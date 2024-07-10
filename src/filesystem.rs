use std::{
    fs::Permissions,
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::SystemTime,
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
    name: String,
    file_type: LinuxFileType,
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct LinuxFileMetadata {
    file_type: Option<LinuxFileType>,
    size: Option<u64>,
    permissions: Option<LinuxPermissions>,
    modified_time: Option<SystemTime>,
    accessed_time: Option<SystemTime>,
    created_time: Option<SystemTime>,
    user_id: Option<u32>,
    user_name: Option<String>,
    group_id: Option<u32>,
    group_name: Option<String>,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct LinuxPermissions: u32 {
        const SET_UID = 0o4000;
        const SET_GID = 0o2000;
        const STICKY_BIT = 0o1000;

        const OWNER_READ = 0o400;
        const OWNER_WRITE = 0o200;
        const OWNER_EXECUTE = 0o100;
        const GROUP_READ = 0o040;
        const GROUP_WRITE = 0o020;
        const GROUP_EXECUTE = 0o010;
        const OTHER_READ = 0o004;
        const OTHER_WRITE = 0o002;
        const OTHER_EXECUTE = 0o001;
    }
}

pub enum LinuxPermissionsError {
    CouldNotExtractMode,
    UnknownBitSet,
}

#[derive(Clone, Copy, Debug)]
pub enum LinuxFileType {
    File,
    Dir,
    Symlink,
    Other,
}

impl LinuxDirEntry {
    pub(crate) fn new(entry_name: String, entry_type: LinuxFileType, entry_path: PathBuf) -> LinuxDirEntry {
        LinuxDirEntry {
            name: entry_name,
            file_type: entry_type,
            path: entry_path,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn file_type(&self) -> LinuxFileType {
        self.file_type
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
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

impl LinuxFileMetadata {
    pub(crate) fn new(
        file_type: Option<LinuxFileType>,
        size: Option<u64>,
        permissions: Option<LinuxPermissions>,
        modified_time: Option<SystemTime>,
        accessed_time: Option<SystemTime>,
        created_time: Option<SystemTime>,
        user_id: Option<u32>,
        user_name: Option<String>,
        group_id: Option<u32>,
        group_name: Option<String>,
    ) -> LinuxFileMetadata {
        LinuxFileMetadata {
            file_type,
            size,
            permissions,
            modified_time,
            accessed_time,
            created_time,
            user_id,
            user_name,
            group_id,
            group_name,
        }
    }

    pub fn file_type(&self) -> Option<LinuxFileType> {
        self.file_type
    }

    pub fn size(&self) -> Option<u64> {
        self.size
    }

    pub fn permissions(&self) -> Option<LinuxPermissions> {
        self.permissions.clone()
    }

    pub fn modified_time(&self) -> Option<SystemTime> {
        self.modified_time
    }

    pub fn accessed_time(&self) -> Option<SystemTime> {
        self.accessed_time
    }

    pub fn created_time(&self) -> Option<SystemTime> {
        self.created_time
    }

    pub fn user_id(&self) -> Option<u32> {
        self.user_id
    }

    pub fn user_name(&self) -> Option<String> {
        self.user_name.clone()
    }

    pub fn group_id(&self) -> Option<u32> {
        self.group_id
    }

    pub fn group_name(&self) -> Option<String> {
        self.group_name.clone()
    }
}

impl TryFrom<Permissions> for LinuxPermissions {
    type Error = LinuxPermissionsError;

    fn try_from(value: Permissions) -> Result<Self, Self::Error> {
        let mode = value
            .mode()
            .try_into()
            .map_err(|_| LinuxPermissionsError::CouldNotExtractMode)?;
        LinuxPermissions::from_bits(mode).ok_or(LinuxPermissionsError::UnknownBitSet)
    }
}

impl Into<Permissions> for LinuxPermissions {
    fn into(self) -> Permissions {
        Permissions::from_mode(self.bits())
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

    async fn create_symlink(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn create_hard_link(&self, source_path: &Path, destination_path: &Path) -> io::Result<()>;

    async fn read_link(&self, link_path: &Path) -> io::Result<PathBuf>;

    async fn set_permissions(&self, path: &Path, permissions: LinuxPermissions) -> io::Result<()>;

    async fn remove_file(&self, path: &Path) -> io::Result<()>;

    async fn create_dir(&self, path: &Path) -> io::Result<()>;

    async fn create_dir_recursively(&self, path: &Path) -> io::Result<()>;

    async fn list_dir(&self, path: &Path) -> io::Result<Vec<LinuxDirEntry>>;

    async fn remove_dir(&self, path: &Path) -> io::Result<()>;

    async fn remove_dir_recursively(&self, path: &Path) -> io::Result<()>;

    async fn get_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata>;

    async fn get_symlink_metadata(&self, path: &Path) -> io::Result<LinuxFileMetadata>;
}
