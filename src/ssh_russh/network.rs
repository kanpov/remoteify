use std::io;

use async_trait::async_trait;

use crate::network::LinuxNetwork;

use super::{event_receiver::RusshGlobalReceiver, RusshLinux};

#[async_trait]
impl<T> LinuxNetwork for RusshLinux<T>
where
    T: RusshGlobalReceiver,
{
    fn is_remote_network(&self) -> bool {
        true
    }

    async fn route_tcp_forward(&mut self, host: &str, port: u32) -> io::Result<u32> {
        let mut handle_instance = self.handle_mutex.lock().await;
        handle_instance
            .tcpip_forward(host, port)
            .await
            .map_err(io::Error::other)
    }
}
