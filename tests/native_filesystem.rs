use common::get_tmp_path;
use lhf::{filesystem::LinuxFilesystem, NativeLinux};
use tokio::{fs::{create_dir, remove_dir, remove_file, File}, io::AsyncWriteExt};
mod common;

static IMPL: NativeLinux = NativeLinux {};

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let path = get_tmp_path();
    assert!(!IMPL.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_file() {
    let path = get_tmp_path();
    File::create_new(&path).await.unwrap().flush().await.unwrap();
    assert!(IMPL.exists(&path).await.expect("Call failed"));
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn exists_is_true_for_existing_dir() {
    let path = get_tmp_path();
    create_dir(&path).await.unwrap();
    assert!(IMPL.exists(&path).await.expect("Call failed"));
    remove_dir(&path).await.unwrap();
}
