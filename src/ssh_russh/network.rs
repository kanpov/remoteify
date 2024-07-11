use std::path::PathBuf;

use async_trait::async_trait;
use russh::client;

use crate::network::{LinuxNetwork, LinuxNetworkError};

use super::RusshLinux;

#[async_trait]
impl<H> LinuxNetwork for RusshLinux<H>
where
    H: client::Handler,
{
    fn is_remote_network(&self) -> bool {
        true
    }

    async fn reverse_forward_tcp(
        &mut self,
        _remote_host: impl Into<String> + Send,
        _remote_port: u16,
        _local_host: impl Into<String> + Send,
        _local_port: u16,
    ) -> Result<(), LinuxNetworkError> {
        Err(LinuxNetworkError::UnsupportedOperation)
    }

    async fn reverse_forward_unix(
        &mut self,
        _remote_socket_path: impl Into<PathBuf> + Send,
        _local_socket_path: impl Into<PathBuf> + Send,
    ) -> Result<(), LinuxNetworkError> {
        Err(LinuxNetworkError::UnsupportedOperation)
    }
}
