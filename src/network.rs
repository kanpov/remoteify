use std::path::PathBuf;

use async_trait::async_trait;

pub enum LinuxNetworkError {
    UnsupportedOperation,
    Other(Box<dyn std::error::Error>),
}

#[async_trait]
pub trait LinuxNetwork {
    fn is_remote_network(&self) -> bool;

    async fn reverse_forward_tcp(
        &mut self,
        remote_host: impl Into<String> + Send,
        remote_port: u16,
        local_host: impl Into<String> + Send,
        local_port: u16,
    ) -> Result<(), LinuxNetworkError>;

    async fn reverse_forward_unix(
        &mut self,
        remote_socket_path: impl Into<PathBuf> + Send,
        local_socket_path: impl Into<PathBuf> + Send,
    ) -> Result<(), LinuxNetworkError>;
}
