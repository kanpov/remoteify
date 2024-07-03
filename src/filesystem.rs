use std::{future::Future, io, path::Path};

pub trait LinuxFilesystem {
    fn exists(&self, path: &Path) -> impl Future<Output = io::Result<bool>>;

    fn create_file(&self, path: &Path) -> impl Future<Output = io::Result<()>>;

    fn write_text_to_file(&self, path: &Path, text: &String) -> impl Future<Output = io::Result<()>>;

    fn write_bytes_to_file(&self, path: &Path, bytes: &[u8]) -> impl Future<Output = io::Result<()>>;
}
