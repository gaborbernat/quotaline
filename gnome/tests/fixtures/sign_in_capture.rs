//! Captures sign-in launch arguments in integration tests.

use std::{env, io::Write, os::unix::net::UnixStream, path::PathBuf};

fn main() {
    let path = PathBuf::from(env::var_os("HOME").unwrap())
        .parent()
        .unwrap()
        .join("terminal-arguments.sock");
    let args = env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    UnixStream::connect(path)
        .unwrap()
        .write_all(format!("{args}\n").as_bytes())
        .unwrap();
}
