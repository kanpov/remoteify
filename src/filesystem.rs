use std::{io, path::Path};

use async_trait::async_trait;

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    async fn create_file(&self, path: &Path) -> io::Result<()>;

    async fn write_text_to_file(&self, path: &Path, text: &String) -> io::Result<()>;

    async fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
}
