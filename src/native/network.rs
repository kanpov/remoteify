use std::path::PathBuf;

use async_trait::async_trait;

use crate::network::{LinuxNetwork, LinuxNetworkError};

use super::NativeLinux;

#[async_trait]
impl LinuxNetwork for NativeLinux {
    fn is_remote_network(&self) -> bool {
        false
    }

    async fn reverse_forward_tcp(
        &mut self,
        _remote_host: impl Into<String> + Send,
        _remote_port: u16,
        _local_host: impl Into<String> + Send,
        _local_port: u16,
    ) -> Result<(), LinuxNetworkError> {
        Ok(())
    }

    async fn reverse_forward_unix(
        &mut self,
        _remote_socket_path: impl Into<PathBuf> + Send,
        _local_socket_path: impl Into<PathBuf> + Send,
    ) -> Result<(), LinuxNetworkError> {
        Ok(())
    }
}
