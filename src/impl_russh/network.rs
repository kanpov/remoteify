use async_trait::async_trait;
use russh::client;

use crate::network::{LinuxNetwork, LinuxNetworkError, LinuxNetworkSocket};

use super::RusshLinux;

#[async_trait]
impl<H> LinuxNetwork for RusshLinux<H>
where
    H: client::Handler,
{
    fn needs_forwarding(&self) -> bool {
        true
    }

    async fn reverse_forward(
        &self,
        _local_socket: LinuxNetworkSocket,
        _remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        Err(LinuxNetworkError::ForwardingNotSupported)
    }

    async fn direct_forward(
        &self,
        _local_socket: LinuxNetworkSocket,
        _remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        Err(LinuxNetworkError::ForwardingNotSupported)
    }
}
