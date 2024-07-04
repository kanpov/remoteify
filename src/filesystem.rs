use std::{io, path::Path};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    async fn create_file(&self, path: &Path) -> io::Result<()>;

    async fn get_file_writer(&self, path: &Path) -> io::Result<impl AsyncWriteExt>;

    async fn get_file_writer_for_append(&self, path: &Path) -> io::Result<impl AsyncWriteExt>;

    async fn get_file_reader(&self, path: &Path) -> io::Result<impl AsyncReadExt>;
}
