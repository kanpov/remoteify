use std::{
    fs::Permissions,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Clone, Copy, Debug)]
pub struct LinuxOpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
}

#[derive(Clone, Debug)]
pub struct LinuxDirEntry {
    entry_name: String,
    entry_type: LinuxDirEntryType,
    entry_path: PathBuf,
}

#[derive(Clone, Copy, Debug)]
pub enum LinuxDirEntryType {
    File,
    Dir,
    Symlink,
    Other,
}

impl LinuxDirEntry {
    pub(crate) fn new(entry_name: String, entry_type: LinuxDirEntryType, entry_path: PathBuf) -> LinuxDirEntry {
        LinuxDirEntry {
            entry_name,
            entry_type,
            entry_path,
        }
    }

    pub fn entry_name(&self) -> String {
        self.entry_name.clone()
    }

    pub fn entry_type(&self) -> LinuxDirEntryType {
        self.entry_type
    }

    pub fn entry_path(&self) -> PathBuf {
        self.entry_path.clone()
    }
}

impl Default for LinuxOpenOptions {
    fn default() -> Self {
        Self {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
        }
    }
}

impl LinuxOpenOptions {
    pub fn new() -> LinuxOpenOptions {
        LinuxOpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
        }
    }

    pub fn is_read(&self) -> bool {
        self.read
    }

    pub fn is_write(&self) -> bool {
        self.write
    }

    pub fn is_append(&self) -> bool {
        self.append
    }

    pub fn is_truncate(&self) -> bool {
        self.truncate
    }

    pub fn is_create(&self) -> bool {
        self.create
    }

    pub fn read(&mut self) -> &mut LinuxOpenOptions {
        self.read = true;
        self
    }

    pub fn write(&mut self) -> &mut LinuxOpenOptions {
        self.write = true;
        self
    }

    pub fn append(&mut self) -> &mut LinuxOpenOptions {
        self.append = true;
        self
    }

    pub fn truncate(&mut self) -> &mut LinuxOpenOptions {
        self.truncate = true;
        self
    }

    pub fn create(&mut self) -> &mut LinuxOpenOptions {
        self.create = true;
        self
    }
}

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &Path) -> io::Result<bool>;

    async fn create_file(&self, path: &Path) -> io::Result<()>;

    async fn open_file(
        &self,
        path: &Path,
        open_options: &LinuxOpenOptions,
    ) -> io::Result<impl AsyncReadExt + AsyncSeekExt + AsyncWriteExt>;

    async fn rename_file(&self, old_path: &Path, new_path: &Path) -> io::Result<()>;

    async fn copy_file(&self, old_path: &Path, new_path: &Path) -> io::Result<Option<u64>>;

    async fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    async fn symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn hardlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf>;

    async fn set_permissions(&self, path: &Path, permissions: Permissions) -> io::Result<()>;

    async fn remove_file(&self, path: &Path) -> io::Result<()>;

    async fn create_dir(&self, path: &Path) -> io::Result<()>;

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()>;

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>>;
}
