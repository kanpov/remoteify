use async_trait::async_trait;
use tokio::fs::{
    canonicalize, copy, create_dir, create_dir_all, hard_link, metadata, read_dir, read_link, remove_dir,
    remove_dir_all, remove_file, rename, set_permissions, symlink, symlink_metadata, try_exists, File, OpenOptions,
};

use super::NativeLinux;
use crate::filesystem::{
    LinuxDirEntry, LinuxFileMetadata, LinuxFileType, LinuxFilesystem, LinuxOpenOptions, LinuxPermissions,
};
use std::{
    fs::{FileType, Metadata, Permissions},
    io,
    os::unix::fs::{MetadataExt, PermissionsExt},
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
        let _ = File::create_new(path).await?;
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

    async fn create_symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        symlink(source_path, destination_path).await
    }

    async fn create_hard_link(&self, source_path: &Path, destination_path: &Path) -> io::Result<()> {
        hard_link(source_path, destination_path).await
    }

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf> {
        read_link(link_path).await
    }

    async fn set_permissions(&self, path: &Path, permissions: LinuxPermissions) -> io::Result<()> {
        set_permissions(path, Permissions::from_mode(permissions.bits())).await
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

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>> {
        let mut read_dir = read_dir(path).await?;

        let mut entries: Vec<LinuxDirEntry> = vec![];
        loop {
            let entry = read_dir.next_entry().await?;

            match entry {
                Some(entry_value) => {
                    let entry_name = entry_value
                        .file_name()
                        .into_string()
                        .map_err(|_| io::Error::other("could not convert entry filename into string"))?;

                    let file_type = entry_value
                        .file_type()
                        .await
                        .map(|entry_type| entry_type.into())
                        .map_err(io::Error::other)?;

                    let entry_path = entry_value.path();

                    entries.push(LinuxDirEntry::new(entry_name, file_type, entry_path));
                }
                None => {
                    break;
                }
            }
        }

        Ok(entries)
    }

    async fn remove_dir(&self, path: &Path) -> io::Result<()> {
        remove_dir(path).await
    }

    async fn remove_dir_recursively(&self, path: &Path) -> io::Result<()> {
        remove_dir_all(path).await
    }

    async fn get_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        let metadata = metadata(path).await?;
        Ok(metadata.into())
    }

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata> {
        let metadata = symlink_metadata(path).await?;
        Ok(metadata.into())
    }
}

impl Into<LinuxFileType> for FileType {
    fn into(self) -> LinuxFileType {
        if self.is_file() {
            return LinuxFileType::File;
        }
        if self.is_dir() {
            return LinuxFileType::Dir;
        }
        if self.is_symlink() {
            return LinuxFileType::Symlink;
        }

        LinuxFileType::Other
    }
}

impl Into<LinuxFileMetadata> for Metadata {
    fn into(self) -> LinuxFileMetadata {
        LinuxFileMetadata::new(
            Some(self.file_type().into()),
            Some(self.size()),
            Some(LinuxPermissions::from_bits_retain(self.permissions().mode())),
            self.modified().ok(),
            self.accessed().ok(),
            self.created().ok(),
            Some(self.uid()),
            None,
            Some(self.gid()),
            None,
        )
    }
}
