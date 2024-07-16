// Modules

#[cfg(feature = "executor")]
pub mod executor;
#[cfg(feature = "filesystem")]
pub mod filesystem;
#[cfg(feature = "network")]
pub mod network;

// Out of the box implementations

#[cfg(feature = "impl-ssh-common")]
#[cfg(feature = "executor")]
pub(crate) mod derive_ext;
#[cfg(feature = "impl-native")]
pub mod impl_native;
#[cfg(feature = "impl-openssh")]
pub mod impl_openssh;
#[cfg(feature = "impl-russh")]
pub mod impl_russh;
