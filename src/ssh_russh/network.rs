use std::io;

use async_trait::async_trait;
use russh::client;

use crate::network::LinuxNetwork;

use super::RusshLinux;

#[async_trait]
impl<T> LinuxNetwork for RusshLinux<T>
where
    T: client::Handler,
{
    fn is_remote(&self) -> bool {
        true
    }

    async fn route_tcp_forward(&mut self, host: &str, port: u32) -> io::Result<u32> {
        let mut handle_instance = self.handle.lock().await;
        handle_instance
            .tcpip_forward(host, port)
            .await
            .map_err(|err| io::Error::other(err))
    }
}
