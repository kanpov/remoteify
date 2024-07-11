use std::path::PathBuf;

use async_trait::async_trait;
use openssh::{ForwardType, Socket};

use crate::network::{LinuxNetwork, LinuxNetworkError};

use super::OpensshLinux;

#[async_trait]
impl LinuxNetwork for OpensshLinux {
    fn is_remote_network(&self) -> bool {
        true
    }

    async fn reverse_forward_tcp(
        &mut self,
        remote_host: impl Into<String> + Send,
        remote_port: u16,
        local_host: impl Into<String> + Send,
        local_port: u16,
    ) -> Result<(), LinuxNetworkError> {
        self.session
            .request_port_forward(
                ForwardType::Remote,
                Socket::TcpSocket {
                    host: remote_host.into().into(),
                    port: remote_port,
                },
                Socket::TcpSocket {
                    host: local_host.into().into(),
                    port: local_port,
                },
            )
            .await
            .map_err(|err| LinuxNetworkError::Other(Box::new(err)))
    }

    async fn reverse_forward_unix(
        &mut self,
        remote_socket_path: impl Into<PathBuf> + Send,
        local_socket_path: impl Into<PathBuf> + Send,
    ) -> Result<(), LinuxNetworkError> {
        self.session
            .request_port_forward(
                ForwardType::Remote,
                Socket::UnixSocket {
                    path: remote_socket_path.into().into(),
                },
                Socket::UnixSocket {
                    path: local_socket_path.into().into(),
                },
            )
            .await
            .map_err(|err| LinuxNetworkError::Other(Box::new(err)))
    }
}
