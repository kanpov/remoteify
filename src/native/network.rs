use async_trait::async_trait;

use crate::network::{LinuxNetwork, LinuxNetworkError, LinuxNetworkSocket};

use super::NativeLinux;

#[async_trait]
impl LinuxNetwork for NativeLinux {
    fn needs_forwarding(&self) -> bool {
        false
    }

    async fn reverse_forward(
        &self,
        _local_socket: LinuxNetworkSocket,
        _remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        Ok(())
    }

    async fn direct_forward(
        &self,
        _local_socket: LinuxNetworkSocket,
        _remote_socket: LinuxNetworkSocket,
    ) -> Result<(), LinuxNetworkError> {
        Ok(())
    }
}
