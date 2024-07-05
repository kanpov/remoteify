use std::{fs::Permissions, io::SeekFrom, os::unix::fs::PermissionsExt, path::Path};

use common::get_tmp_path;
use lhf::{filesystem::LinuxFilesystem, native::NativeLinux};
use tokio::{
    fs::{
        create_dir, metadata, read_to_string, remove_dir, remove_file, symlink, symlink_metadata,
        try_exists, write, File,
    },
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};
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
    File::create_new(&path)
        .await
        .unwrap()
        .flush()
        .await
        .unwrap();
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

#[tokio::test]
async fn file_open_write_non_truncate() {
    let path = get_tmp_path();
    write(&path, "certain content").await.unwrap();
    let mut writer = IMPL
        .file_open_write(&path, false)
        .await
        .expect("Call failed");
    writer.write_all(b"b").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "bertain content");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_write_truncate() {
    let path = get_tmp_path();
    write(&path, "sample").await.unwrap();
    let mut writer = IMPL
        .file_open_write(&path, true)
        .await
        .expect("Call failed");
    writer.write_all(b"new").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "new");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_append() {
    let path = get_tmp_path();
    write(&path, "first").await.unwrap();
    let mut writer = IMPL.file_open_append(&path).await.expect("Call failed");
    writer.write_all(b"second").await.unwrap();
    assert_eq!(read_to_string(&path).await.unwrap(), "firstsecond");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_read() {
    let path = get_tmp_path();
    write(&path, "content").await.unwrap();
    let mut reader = IMPL.file_open_read(&path).await.expect("Call failed");
    let mut buf = String::new();
    reader.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_read_write_non_truncate() {
    let path = get_tmp_path();
    write(&path, "").await.unwrap();
    let mut read_writer = IMPL
        .file_open_read_write(&path, false)
        .await
        .expect("Call failed");
    read_writer.write_all(b"TEST").await.unwrap();
    let mut buf = String::new();
    read_writer.seek(SeekFrom::Start(0)).await.unwrap();
    read_writer.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "TEST");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_read_write_truncate() {
    let path = get_tmp_path();
    write(&path, "content").await.unwrap();
    let mut read_writer = IMPL
        .file_open_read_write(&path, true)
        .await
        .expect("Call failed");
    read_writer.write_all(b"con").await.unwrap();
    let mut buf = String::new();
    read_writer.seek(SeekFrom::Start(0)).await.unwrap();
    read_writer.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "con");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn file_open_read_append() {
    let path = get_tmp_path();
    write(&path, "content").await.unwrap();
    let mut read_writer = IMPL
        .file_open_read_append(&path)
        .await
        .expect("Call failed");
    read_writer.write_all(b"next").await.unwrap();
    let mut buf = String::new();
    read_writer.seek(SeekFrom::Start(0)).await.unwrap();
    read_writer.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "contentnext");
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn create_file_should_persist() {
    let path = get_tmp_path();
    IMPL.create_file(&path).await.expect("Call failed");
    assert!(try_exists(&path).await.unwrap());
    remove_file(&path).await.unwrap();
}

#[tokio::test]
async fn rename_file_should_persist() {
    let old_path = get_tmp_path();
    let new_path = get_tmp_path();
    File::create_new(&old_path).await.unwrap();
    IMPL.rename_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    assert!(!try_exists(&old_path).await.unwrap());
    assert!(try_exists(&new_path).await.unwrap());
    remove_file(&new_path).await.unwrap();
}

#[tokio::test]
async fn copy_file_should_persist() {
    let old_path = get_tmp_path();
    let new_path = get_tmp_path();
    write(&old_path, "content").await.unwrap();
    IMPL.copy_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    assert_eq!(read_to_string(&new_path).await.unwrap(), "content");
    remove_file(&old_path).await.unwrap();
    remove_file(&new_path).await.unwrap();
}

#[tokio::test]
async fn canonicalize_should_perform_operation() {
    let canonicalized_path = IMPL
        .canonicalize(Path::new("/tmp/../tmp/../tmp"))
        .await
        .expect("Call failed");
    assert_eq!(canonicalized_path.to_str().unwrap(), "/tmp");
}

#[tokio::test]
async fn symlink_should_establish_link() {
    let src_path = get_tmp_path();
    let dst_path = get_tmp_path();
    write(&src_path, "").await.unwrap();
    IMPL.symlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    assert!(try_exists(&dst_path).await.unwrap());
    assert!(symlink_metadata(&dst_path).await.is_ok());
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn hard_link_should_establish_link() {
    let src_path = get_tmp_path();
    let dst_path = get_tmp_path();
    write(&src_path, "content").await.unwrap();
    IMPL.hardlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    assert!(try_exists(&dst_path).await.unwrap());
    assert_eq!(read_to_string(&dst_path).await.unwrap(), "content");
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn read_link_should_return_correct_location() {
    let src_path = get_tmp_path();
    let dst_path = get_tmp_path();
    write(&src_path, "").await.unwrap();
    symlink(&src_path, &dst_path).await.unwrap();
    assert_eq!(src_path, IMPL.read_link(&dst_path).await.unwrap());
    remove_file(&src_path).await.unwrap();
    remove_file(&dst_path).await.unwrap();
}

#[tokio::test]
async fn set_permissions_should_perform_update() {
    let path = get_tmp_path();
    write(&path, "content").await.unwrap();
    IMPL.set_permissions(&path, Permissions::from_mode(777))
        .await
        .unwrap();
    let meta = metadata(&path).await.unwrap();
    assert_eq!(meta.permissions().mode(), 33545);
    remove_file(&path).await.unwrap();
}
