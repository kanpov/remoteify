use std::{fs::Permissions, os::unix::fs::PermissionsExt, path::Path};

use common::{conv_path, gen_tmp_path, TestData};
use lhf::filesystem::{LinuxFilesystem, LinuxOpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod common;

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let test_data = TestData::setup().await;
    let path = gen_tmp_path();
    assert!(!test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_file() {
    let test_data = TestData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create(conv_path(&path)).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existent_dir() {
    let test_data = TestData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create_dir(conv_path(&path)).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn open_file_with_read_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    handle.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("content").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    handle.write_all(b"CON").await.unwrap();
    test_data.assert_file(&path, "CONtent").await;
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("first").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().append())
        .await
        .expect("Call failed");
    handle.write_all(b"second").await.unwrap();
    test_data.assert_file(&path, "firstsecond").await;
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("current").await;
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    handle.write_all(b"next").await.unwrap();
    test_data.assert_file(&path, "next").await;
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let test_data = TestData::setup().await;
    let path = gen_tmp_path();
    let mut handle = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().create())
        .await
        .expect("Call failed");
    handle.write_all(b"content").await.unwrap();
    test_data.assert_file(&path, "content").await;
}

#[tokio::test]
async fn create_file_should_persist() {
    let test_data = TestData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_file(&path).await.expect("Call failed");
    assert!(test_data.sftp.try_exists(conv_path(&path)).await.unwrap());
}

#[tokio::test]
async fn rename_file_should_perform_change() {
    let test_data = TestData::setup().await;
    let old_path = test_data.init_file("some content").await;
    let new_path = gen_tmp_path();
    test_data
        .implementation
        .rename_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    assert!(!test_data.sftp.try_exists(conv_path(&old_path)).await.unwrap());
    assert!(test_data.sftp.try_exists(conv_path(&new_path)).await.unwrap());
    test_data.assert_file(&new_path, "some content").await;
}

#[tokio::test]
async fn copy_file_should_perform_change() {
    let test_data = TestData::setup().await;
    let old_path = test_data.init_file("content").await;
    let new_path = gen_tmp_path();
    test_data
        .implementation
        .copy_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&old_path, "content").await;
    test_data.assert_file(&new_path, "content").await;
}

#[tokio::test]
async fn canonicalize_should_perform_transformation() {
    let test_data = TestData::setup().await;
    assert_eq!(
        test_data
            .implementation
            .canonicalize(Path::new("/tmp/../tmp/../tmp"))
            .await
            .expect("Call failed")
            .to_str()
            .unwrap(),
        "/tmp"
    );
}

#[tokio::test]
async fn symlink_should_perform_linking() {
    let test_data = TestData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .implementation
        .symlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    assert_eq!(
        test_data.sftp.read_link(conv_path(&dst_path)).await.unwrap(),
        src_path.to_str().unwrap()
    );
}

#[tokio::test]
async fn hardlink_should_perform_linking() {
    let test_data = TestData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .implementation
        .hardlink(&src_path, &dst_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&src_path, "content").await;
    test_data.assert_file(&dst_path, "content").await;
}

#[tokio::test]
async fn read_link_should_return_correct_source_path() {
    let test_data = TestData::setup().await;
    let src_path = test_data.init_file("content").await;
    let dst_path = gen_tmp_path();
    test_data
        .sftp
        .symlink(conv_path(&src_path), conv_path(&dst_path))
        .await
        .unwrap();
    assert_eq!(
        test_data
            .implementation
            .read_link(&dst_path)
            .await
            .expect("Call failed"),
        src_path
    );
}

#[tokio::test]
async fn set_permissions_should_perform_change() {
    let test_data = TestData::setup().await;
    let path = test_data.init_file("content").await;
    test_data
        .implementation
        .set_permissions(&path, Permissions::from_mode(777))
        .await
        .expect("Call failed");
    assert_eq!(
        test_data
            .sftp
            .metadata(conv_path(&path))
            .await
            .unwrap()
            .permissions
            .unwrap(),
        33545
    );
}
