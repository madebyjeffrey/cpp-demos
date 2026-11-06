//! Target selection for the native desktop integration suite.
#[cfg(target_os = "macos")]
#[path = "macos/desktop.rs"]
mod suite;

fn main() {
    #[cfg(target_os = "macos")]
    suite::run();
    #[cfg(not(target_os = "macos"))]
    println!("Desktop tests require macOS 26+");
}
