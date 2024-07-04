use std::io::SeekFrom;

use common::get_tmp_path;
use lhf::{filesystem::LinuxFilesystem, NativeLinux};
use tokio::{
    fs::{create_dir, read_to_string, remove_dir, remove_file, write, File},
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
    let mut read_writer = IMPL.file_open_read_write(&path, true).await.expect("Call failed");
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
    let mut read_writer = IMPL.file_open_read_append(&path).await.expect("Call failed");
    read_writer.write_all(b"next").await.unwrap();
    let mut buf = String::new();
    read_writer.seek(SeekFrom::Start(0)).await.unwrap();
    read_writer.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "contentnext");
    remove_file(&path).await.unwrap();
}
