pub mod filesystem;

#[cfg(feature = "native")]
mod native;

#[cfg(feature = "ssh_russh")]
mod ssh_russh;

#[cfg(feature = "native")]
pub use native::NativeLinux;

#[cfg(feature = "ssh_russh")]
pub use ssh_russh::RusshLinux;
