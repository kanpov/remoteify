use std::io;

use async_trait::async_trait;

use crate::network::LinuxNetwork;

use super::{event_receiver::RusshEventReceiver, RusshLinux};

#[async_trait]
impl<T> LinuxNetwork for RusshLinux<T>
where
    T: RusshEventReceiver,
{
    fn is_remote_network(&self) -> bool {
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
