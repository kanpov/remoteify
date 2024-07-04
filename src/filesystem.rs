use std::{io, path::Path};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    async fn create_file(&self, path: &Path) -> io::Result<()>;

    async fn file_open_write(&self, path: &Path, truncate: bool) -> io::Result<impl AsyncWriteExt>;

    async fn file_open_append(&self, path: &Path, truncate: bool)
        -> io::Result<impl AsyncWriteExt>;

    async fn file_open_read(&self, path: &Path, truncate: bool) -> io::Result<impl AsyncReadExt>;

    async fn file_open_read_write(
        &self,
        path: &Path,
        truncate: bool,
    ) -> io::Result<impl AsyncReadExt + AsyncWriteExt>;

    async fn rename(&self, old_path: &Path, new_path: &Path) -> io::Result<()>;

    async fn copy(&self, old_path: &Path, new_path: &Path) -> io::Result<u32>;
}
