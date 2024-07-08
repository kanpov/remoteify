use std::error::Error;

use async_trait::async_trait;

pub enum LinuxNetworkError {
    UnsupportedOperation,
    Other(Box<dyn Error + Send>),
}

#[async_trait]
pub trait LinuxNetwork {
    fn is_remote_network(&self) -> bool;

    async fn reverse_forward_tcp(
        &mut self,
        host: impl Into<String> + Send,
        port: u32,
    ) -> Result<u32, LinuxNetworkError>;

    async fn reverse_forward_unix(
        &mut self,
        socket_path: impl Into<String> + Send,
    ) -> Result<String, LinuxNetworkError>;
}
