use async_trait::async_trait;
use openssh::{ForwardType, Socket};

use crate::network::{LinuxNetwork, LinuxNetworkError, LinuxNetworkSocket};

use super::OpensshLinux;

#[async_trait]
impl LinuxNetwork for OpensshLinux {
    fn needs_forwarding(&self) -> bool {
        true
    }

    async fn reverse_forward(
        &self,
        local_socket: LinuxNetworkSocket,
        remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        self.session
            .request_port_forward(
                ForwardType::Remote,
                conv_socket(remote_socket),
                conv_socket(local_socket),
            )
            .await
            .map_err(|err| LinuxNetworkError::Other(Box::new(err)))
    }

    async fn direct_forward(
        &self,
        local_socket: LinuxNetworkSocket,
        remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        self.session
            .request_port_forward(
                ForwardType::Local,
                conv_socket(local_socket),
                conv_socket(remote_socket),
            )
            .await
            .map_err(|err| LinuxNetworkError::Other(Box::new(err)))
    }
}

fn conv_socket<'a>(socket: LinuxNetworkSocket) -> Socket<'a> {
    match socket {
        LinuxNetworkSocket::Tcp { host, port } => Socket::TcpSocket {
            host: host.into(),
            port,
        },
        LinuxNetworkSocket::Unix { socket_path } => Socket::UnixSocket {
            path: socket_path.into(),
        },
    }
}
