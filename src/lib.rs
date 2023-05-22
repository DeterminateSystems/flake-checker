mod error;
mod flake;

pub use error::FlakeCheckerError;
pub use flake::{FlakeLock, Summary};

// MAYBE: re-introduce logging
fn _warn(path: &str, message: &str) {
    println!("::warning file={path}::{message}");
}
