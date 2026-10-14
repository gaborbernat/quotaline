//! Reads Antigravity process arguments and listening sockets through libproc.

use libproc::libproc::{
    bsd_info::BSDInfo,
    file_info::{ListFDs, ProcFDType, pidfdinfo},
    net_info::{SocketFDInfo, SocketInfoKind, TcpSIState},
    proc_pid::{listpidinfo, pidinfo, pidpath},
};
use libproc::processes::{ProcFilter, pids_by_type};
use quotaline_antigravity::local;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: i32,
    pub path: String,
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalServer {
    pub pid: i32,
    pub port: u16,
    pub csrf_token: Option<String>,
}

pub fn discover_servers(
    processes: impl IntoIterator<Item = ProcessEntry>,
    mut ports: impl FnMut(i32) -> Vec<u16>,
) -> Vec<LocalServer> {
    processes
        .into_iter()
        .filter_map(|process| {
            let arguments = process.arguments.iter().map(String::as_str).collect::<Vec<_>>();
            let role = local::role(&process.path, &arguments)?;
            let csrf_token = match role {
                local::LocalRole::Cli => None,
                local::LocalRole::LanguageServer { csrf_token } => Some(csrf_token),
            };
            let mut listening = ports(process.pid);
            listening.sort_unstable();
            Some(listening.into_iter().map(move |port| LocalServer {
                pid: process.pid,
                port,
                csrf_token: csrf_token.clone(),
            }))
        })
        .flatten()
        .collect()
}

#[cfg_attr(test, mockall::automock)]
pub trait ProcessCalls: Send + Sync {
    fn pids(&self) -> Vec<i32>;
    fn path(&self, pid: i32) -> Option<String>;
    fn argument_bytes(&self, pid: i32) -> Vec<u8>;
    fn ports(&self, pid: i32) -> Vec<u16>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub fd: i32,
    pub socket: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Socket {
    Other,
    Tcp { listening: bool, port: u16 },
}

#[cfg_attr(test, mockall::automock)]
pub trait NativeCalls: Send + Sync {
    fn argument_size(&self, pid: i32) -> Option<usize>;
    fn read_arguments(&self, pid: i32, bytes: &mut [u8]) -> Option<usize>;
    fn fd_count(&self, pid: i32) -> Option<usize>;
    fn descriptors(&self, pid: i32, capacity: usize) -> Option<Vec<Descriptor>>;
    fn socket(&self, pid: i32, fd: i32) -> Option<Socket>;
}

pub struct LibprocCalls;

impl NativeCalls for LibprocCalls {
    fn argument_size(&self, pid: i32) -> Option<usize> {
        let mut names = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
        let mut size = 0;
        (unsafe {
            libc::sysctl(
                names.as_mut_ptr(),
                3,
                std::ptr::null_mut(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        } == 0)
            .then_some(size)
    }

    fn read_arguments(&self, pid: i32, bytes: &mut [u8]) -> Option<usize> {
        let mut names = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
        let mut size = bytes.len();
        (unsafe {
            libc::sysctl(
                names.as_mut_ptr(),
                3,
                bytes.as_mut_ptr().cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        } == 0)
            .then_some(size)
    }

    fn fd_count(&self, pid: i32) -> Option<usize> {
        pidinfo::<BSDInfo>(pid, 0).ok().map(|info| info.pbi_nfiles as usize)
    }

    fn descriptors(&self, pid: i32, capacity: usize) -> Option<Vec<Descriptor>> {
        listpidinfo::<ListFDs>(pid, capacity).ok().map(|fds| {
            fds.into_iter()
                .map(|fd| Descriptor {
                    fd: fd.proc_fd,
                    socket: matches!(ProcFDType::from(fd.proc_fdtype), ProcFDType::Socket),
                })
                .collect()
        })
    }

    fn socket(&self, pid: i32, fd: i32) -> Option<Socket> {
        let info = pidfdinfo::<SocketFDInfo>(pid, fd).ok()?;
        if !matches!(SocketInfoKind::from(info.psi.soi_kind), SocketInfoKind::Tcp) {
            return Some(Socket::Other);
        }
        let tcp = unsafe { info.psi.soi_proto.pri_tcp };
        Some(Socket::Tcp {
            listening: matches!(TcpSIState::from(tcp.tcpsi_state), TcpSIState::Listen),
            port: u16::from_be(tcp.tcpsi_ini.insi_lport as u16),
        })
    }
}

pub struct SystemCalls(Box<dyn NativeCalls>);

impl Default for SystemCalls {
    fn default() -> Self {
        Self(Box::new(LibprocCalls))
    }
}

impl SystemCalls {
    pub fn with_native(native: impl NativeCalls + 'static) -> Self {
        Self(Box::new(native))
    }
}

impl ProcessCalls for SystemCalls {
    fn pids(&self) -> Vec<i32> {
        pids_by_type(ProcFilter::ByUID {
            uid: unsafe { libc::geteuid() },
        })
        .unwrap_or_default()
        .into_iter()
        .map(|pid| pid as i32)
        .collect()
    }

    fn path(&self, pid: i32) -> Option<String> {
        pidpath(pid).ok()
    }

    fn argument_bytes(&self, pid: i32) -> Vec<u8> {
        let Some(size) = self.0.argument_size(pid) else {
            return Vec::new();
        };
        if size == 0 {
            return Vec::new();
        }
        let mut bytes = vec![0; size];
        let Some(size) = self.0.read_arguments(pid, &mut bytes) else {
            return Vec::new();
        };
        bytes.truncate(size);
        bytes
    }

    fn ports(&self, pid: i32) -> Vec<u16> {
        let Some(capacity) = self.0.fd_count(pid) else {
            return Vec::new();
        };
        let Some(fds) = self.0.descriptors(pid, capacity) else {
            return Vec::new();
        };
        fds.into_iter()
            .filter(|fd| fd.socket)
            .filter_map(|fd| self.0.socket(pid, fd.fd))
            .filter_map(|socket| match socket {
                Socket::Tcp { listening: true, port } => Some(port),
                _ => None,
            })
            .collect()
    }
}

pub struct Scanner(Box<dyn ProcessCalls>);

impl Scanner {
    pub fn system() -> Self {
        Self(Box::new(SystemCalls::default()))
    }

    pub fn with_calls(calls: impl ProcessCalls + 'static) -> Self {
        Self(Box::new(calls))
    }

    pub fn processes(&self) -> Vec<ProcessEntry> {
        self.0
            .pids()
            .into_iter()
            .filter_map(|pid| {
                let path = self.0.path(pid)?;
                local::candidate(&path).then(|| ProcessEntry {
                    pid,
                    path,
                    arguments: parse_arguments(&self.0.argument_bytes(pid)),
                })
            })
            .collect()
    }

    pub fn listening_ports(&self, pid: i32) -> Vec<u16> {
        self.0.ports(pid)
    }

    pub fn discover_servers(&self) -> Vec<LocalServer> {
        discover_servers(self.processes(), |pid| self.listening_ports(pid))
    }
}

fn parse_arguments(bytes: &[u8]) -> Vec<String> {
    let Some(count) = bytes.get(..4) else { return Vec::new() };
    let count = i32::from_ne_bytes(count.try_into().unwrap());
    if count <= 0 {
        return Vec::new();
    }
    let rest = &bytes[4..];
    let Some(path_end) = rest.iter().position(|byte| *byte == 0) else {
        return Vec::new();
    };
    rest[path_end + 1..]
        .split(|byte| *byte == 0)
        .skip_while(|argument| argument.is_empty())
        .take(count as usize)
        .map(|argument| String::from_utf8_lossy(argument).into_owned())
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/process/tests.rs"]
mod tests;
