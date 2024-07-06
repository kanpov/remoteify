use std::io;

use async_trait::async_trait;

use crate::network::LinuxNetwork;

use super::NativeLinux;

#[async_trait]
impl LinuxNetwork for NativeLinux {
    fn is_remote(&self) -> bool {
        false
    }

    async fn route_tcp_forward(&mut self, _host: &str, port: u32) -> io::Result<u32> {
        Ok(port)
    }
}
