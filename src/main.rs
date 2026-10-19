#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows;

fn main() {
    #[cfg(windows)]
    if let Err(error) = windows::run() {
        windows::show_error(&error);
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    {
        eprintln!(
            "logibar requires Windows 10 or 11; portable parsing tests use cargo test --lib --test database"
        );
        std::process::exit(1);
    }
}
