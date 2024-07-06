use std::io;

use async_trait::async_trait;

#[async_trait]
pub trait LinuxNetwork {
    fn is_remote_network(&self) -> bool;

    async fn route_tcp_forward(&mut self, host: &str, port: u32) -> io::Result<u32>;
}
