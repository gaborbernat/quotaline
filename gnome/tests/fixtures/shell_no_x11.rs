//! Launches a Shell test command without an inherited X11 display.

use std::process::Command;

fn main() {
    let mut args = std::env::args_os();
    args.next();
    let program = args.next().expect("GNOME Shell command");
    let monitor = std::env::var_os("TMB_VIRTUAL_MONITOR");
    let mut next_is_monitor = false;
    let arguments = args.map(|arg| {
        if next_is_monitor {
            next_is_monitor = false;
            monitor.clone().unwrap_or(arg)
        } else {
            next_is_monitor = arg == "--virtual-monitor";
            arg
        }
    });
    match Command::new(program).args(arguments).arg("--no-x11").status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("GNOME Shell exited with {status}");
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("Cannot start GNOME Shell: {error}");
            std::process::exit(1);
        }
    }
}
