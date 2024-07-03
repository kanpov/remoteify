use std::path::Path;

use russh::client;
use tokio::io;

use crate::filesystem::LinuxFilesystem;

use super::SshLinux;

impl<T> LinuxFilesystem for SshLinux<T> where T : client::Handler {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        todo!()
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn write_text_to_file(&self, path: &Path, text: &String) -> io::Result<()> {
        todo!()
    }

    async fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        todo!()
    }
}
