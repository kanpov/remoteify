use async_trait::async_trait;
use tokio::fs::{
    canonicalize, copy, hard_link, read_link, rename, set_permissions, symlink, try_exists, File,
    OpenOptions,
};

use super::NativeLinux;
use crate::filesystem::LinuxFilesystem;
use std::{
    fs::Permissions,
    io,
    path::{Path, PathBuf},
};

#[async_trait]
impl LinuxFilesystem for NativeLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        try_exists(&path).await
    }

    async fn file_open_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().write(true), truncate).await
    }

    async fn file_open_append(&self, path: &Path) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().append(true), false).await
    }

    async fn file_open_read(&self, path: &Path) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().read(true), false).await
    }

    async fn file_open_read_write(&self, path: &Path, truncate: bool) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().read(true).write(true), truncate).await
    }

    async fn file_open_read_append(&self, path: &Path) -> io::Result<File> {
        internal_open_file(path, OpenOptions::new().read(true).append(true), false).await
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        let file = match File::create_new(path).await {
            Ok(file) => file,
            Err(err) => return Err(err),
        };
        drop(file);
        Ok(())
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        rename(old_path, new_path).await
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        match copy(old_path, new_path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) => Err(err),
        }
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        canonicalize(path).await
    }

    async fn symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        symlink(source_path, destination_path).await
    }

    async fn hardlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        hard_link(source_path, destination_path).await
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        read_link(link_path).await
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
        set_permissions(path, permissions).await
    }
}

async fn internal_open_file(
    path: &Path,
    open_options: &mut OpenOptions,
    truncate: bool,
) -> io::Result<File> {
    if truncate {
        open_options.truncate(true).open(path).await
    } else {
        open_options.open(path).await
    }
}
