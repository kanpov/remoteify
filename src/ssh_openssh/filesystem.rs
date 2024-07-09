use std::{
    fs::Permissions,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use openssh_sftp_client::file::{File, OpenOptions, TokioCompatFile};

use crate::filesystem::{LinuxDirEntry, LinuxFileMetadata, LinuxFilesystem, LinuxOpenOptions};

use super::OpensshLinux;

#[async_trait]
impl LinuxFilesystem for OpensshLinux {
    async fn exists(&self, path: &Path) -> io::Result<bool> {
        todo!()
    }

    async fn create_file(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn open_file(&self, path: &Path, open_options: &LinuxOpenOptions) -> io::Result<TokioCompatFile> {
        todo!()
    }

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>> {
        todo!()
    }

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        todo!()
    }

    async fn create_symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn create_hard_link(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        todo!()
    }

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()> {
        todo!()
    }

    async fn remove_file(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn create_dir(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>> {
        todo!()
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn remove_dir_recursively(&self, path: &Path) -> io::Result<()> {
        todo!()
    }

    async fn get_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        todo!()
    }

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        todo!()
    }
}
