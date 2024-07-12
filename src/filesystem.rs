use std::{
    ffi::{OsStr, OsString},
    fs::Permissions,
    io,
    os::unix::fs::PermissionsExt,
    time::SystemTime,
};

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinuxOpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LinuxDirEntry {
    pub name: OsString,
    pub file_type: LinuxFileType,
    pub path: OsString,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LinuxFileMetadata {
    pub file_type: Option<LinuxFileType>,
    pub size: Option<u64>,
    pub permissions: Option<LinuxPermissions>,
    pub modified_time: Option<SystemTime>,
    pub accessed_time: Option<SystemTime>,
    pub created_time: Option<SystemTime>,
    pub user_id: Option<u32>,
    pub user_name: Option<String>,
    pub group_id: Option<u32>,
    pub group_name: Option<String>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPermissionsUnknownBitSetError {
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinuxFileType {
    File,
    Dir,
    Symlink,
    Other,
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

impl TryFrom<Permissions> for LinuxPermissions {
    type Error = LinuxPermissionsUnknownBitSetError;

    fn try_from(value: Permissions) -> Result<Self, Self::Error> {
        let mode = value.mode();
        LinuxPermissions::from_bits(mode).ok_or(LinuxPermissionsUnknownBitSetError { mode })
    }
}

impl From<LinuxPermissions> for Permissions {
    fn from(value: LinuxPermissions) -> Self {
        Permissions::from_mode(value.bits())
    }
}

#[async_trait]
pub trait LinuxFilesystem {
    async fn exists(&self, path: &OsStr) -> io::Result<bool>;

    async fn create_file(&self, path: &OsStr) -> io::Result<()>;

    async fn open_file(
        &self,
        path: &OsStr,
        open_options: &LinuxOpenOptions,
    ) -> io::Result<impl AsyncReadExt + AsyncWriteExt + AsyncSeekExt>;

    async fn rename_file(&self, old_path: &OsStr, new_path: &OsStr) -> io::Result<()>;

    async fn copy_file(&self, old_path: &OsStr, new_path: &OsStr) -> io::Result<Option<u64>>;

    async fn canonicalize(&self, path: &OsStr) -> io::Result<OsString>;

    async fn create_symlink(&self, source_path: &OsStr, destination_path: &OsStr) -> io::Result<()>;

    async fn create_hard_link(&self, source_path: &OsStr, destination_path: &OsStr) -> io::Result<()>;

    async fn read_link(&self, path: &OsStr) -> io::Result<OsString>;

    async fn set_permissions(&self, path: &OsStr, permissions: LinuxPermissions) -> io::Result<()>;

    async fn remove_file(&self, path: &OsStr) -> io::Result<()>;

    async fn create_dir(&self, path: &OsStr) -> io::Result<()>;

    async fn create_dir_recursively(&self, path: &OsStr) -> io::Result<()>;

    async fn list_dir(&self, path: &OsStr) -> io::Result<Vec<LinuxDirEntry>>;

    async fn remove_dir(&self, path: &OsStr) -> io::Result<()>;

    async fn remove_dir_recursively(&self, path: &OsStr) -> io::Result<()>;

    async fn get_metadata(&self, path: &OsStr) -> io::Result<LinuxFileMetadata>;

    async fn get_symlink_metadata(&self, path: &OsStr) -> io::Result<LinuxFileMetadata>;
}
