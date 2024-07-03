use std::{future::Future, io, path::Path};

pub trait LinuxFilesystem {
    fn exists(&self, path: &Path) -> impl Future<Output = io::Result<bool>>;
}
