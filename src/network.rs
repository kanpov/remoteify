use std::path::PathBuf;

use async_trait::async_trait;

#[derive(Debug)]
pub enum LinuxNetworkError {
    ForwardingNotSupported,
    Other(Box<dyn std::error::Error>),
}

pub enum LinuxNetworkSocket {
    Tcp { host: String, port: u16 },
    Unix { socket_path: PathBuf },
}

#[async_trait]
pub trait LinuxNetwork {
    fn needs_forwarding(&self) -> bool;

    async fn reverse_forward(
        &self,
        local_socket: LinuxNetworkSocket,
        remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError>;

    async fn direct_forward(
        &self,
        local_socket: LinuxNetworkSocket,
        remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError>;
}
