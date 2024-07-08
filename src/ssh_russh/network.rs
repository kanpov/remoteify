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
        host: impl Into<String> + Send,
        port: u32,
    ) -> Result<u32, LinuxNetworkError> {
        let mut handle = self.handle_mutex.lock().await;
        handle
            .tcpip_forward(host, port)
            .await
            .map_err(|err| LinuxNetworkError::Other(Box::new(err)))
    }

    async fn reverse_forward_unix(
        &mut self,
        _socket_path: impl Into<String> + Send,
    ) -> Result<String, LinuxNetworkError> {
        Err(LinuxNetworkError::UnsupportedOperation)
    }
}
