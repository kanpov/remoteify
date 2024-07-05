use async_trait::async_trait;
use tokio::fs::{
    canonicalize, copy, create_dir, create_dir_all, hard_link, read_dir, read_link, remove_dir, remove_dir_all,
    remove_file, rename, set_permissions, symlink, try_exists, File, OpenOptions,
};

use super::NativeLinux;
use crate::filesystem::{LinuxDirEntry, LinuxDirEntryType, LinuxFilesystem, LinuxOpenOptions};
use std::{
    fs::{FileType, Permissions},
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

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>> {
        let mut read_dir = match read_dir(path).await {
            Ok(read_dir) => read_dir,
            Err(err) => return Err(err),
        };

        let mut entries: Vec<LinuxDirEntry> = vec![];
        loop {
            let entry = match read_dir.next_entry().await {
                Ok(entry) => entry,
                Err(err) => return Err(err),
            };

            match entry {
                Some(entry_value) => {
                    let entry_name = match entry_value.file_name().into_string() {
                        Ok(entry_name) => entry_name,
                        Err(_) => return Err(io::Error::other("could not convert os_str into str")),
                    };

                    let entry_type = match entry_value.file_type().await {
                        Ok(entry_type) => entry_type.into(),
                        Err(err) => return Err(io::Error::other(err)),
                    };

                    let entry_path = entry_value.path();

                    entries.push(LinuxDirEntry::new(entry_name, entry_type, entry_path));
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
}

impl From<FileType> for LinuxDirEntryType {
    fn from(value: FileType) -> Self {
        if value.is_file() {
            return LinuxDirEntryType::File;
        }
        if value.is_dir() {
            return LinuxDirEntryType::Dir;
        }
        if value.is_symlink() {
            return LinuxDirEntryType::Symlink;
        }

        LinuxDirEntryType::Other
    }
}
