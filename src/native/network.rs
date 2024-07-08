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
        _host: impl Into<String> + Send,
        port: u32,
    ) -> Result<u32, LinuxNetworkError> {
        Ok(port)
    }

    async fn reverse_forward_unix(
        &mut self,
        socket_path: impl Into<String> + Send,
    ) -> Result<String, LinuxNetworkError> {
        Ok(socket_path.into())
    }
}
