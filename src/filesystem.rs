use std::{
    fs::Permissions,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt};

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    async fn create_file(&self, path: &Path) -> io::Result<()>;

    async fn file_open_write(&self, path: &Path, truncate: bool) -> io::Result<impl AsyncWriteExt>;

    async fn file_open_append(&self, path: &Path) -> io::Result<impl AsyncWriteExt>;

    async fn file_open_read(&self, path: &Path) -> io::Result<impl AsyncReadExt + AsyncSeekExt>;

    async fn file_open_read_write(
        &self,
        path: &Path,
        truncate: bool,
    ) -> io::Result<impl AsyncReadExt + AsyncSeekExt + AsyncWriteExt>;

    async fn file_open_read_append(
        &self,
        path: &Path,
    ) -> io::Result<impl AsyncReadExt + AsyncSeekExt + AsyncWriteExt>;

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()>;

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>>;

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    async fn symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn hardlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()>;
}
