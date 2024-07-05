use async_trait::async_trait;
use tokio::fs::{
    canonicalize, copy, create_dir, create_dir_all, hard_link, read_link, remove_file, rename, set_permissions,
    symlink, try_exists, File, OpenOptions,
};

use super::NativeLinux;
use crate::filesystem::{LinuxFilesystem, LinuxOpenOptions};
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

    async fn open_file(&self, path: &Path, open_options: &LinuxOpenOptions) -> io::Result<File> {
        let mut final_options = OpenOptions::new();
        if open_options.is_read() {
            final_options.read(true);
        }
        if open_options.is_write() {
            final_options.write(true);
        }
        if open_options.is_append() {
            final_options.append(true);
        }
        if open_options.is_truncate() {
            final_options.truncate(true);
        }
        if open_options.is_create() {
            final_options.create(true);
        }

        final_options.open(path).await
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

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        remove_file(path).await
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        create_dir(path).await
    }

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()> {
        create_dir_all(path).await
    }
}
