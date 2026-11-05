//! Target selection for the native fullscreen integration suite.
#[cfg(target_os = "macos")]
#[path = "macos/fullscreen.rs"]
mod suite;

fn main() {
    #[cfg(target_os = "macos")]
    suite::run();
    #[cfg(not(target_os = "macos"))]
    println!("Fullscreen tests require macOS 26+");
}
