use common::{gen_tmp_path, OpensshData};
use remoteify::filesystem::{LinuxFilesystem, LinuxOpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

mod common;

#[tokio::test]
async fn exists_is_false_for_missing_item() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    assert!(!test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_file() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.create(&path).await.unwrap().close().await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn exists_is_true_for_existing_dir() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().create_dir(&path).await.unwrap();
    assert!(test_data.implementation.exists(&path).await.expect("Call failed"));
}

#[tokio::test]
async fn open_file_with_read_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    let mut reader = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().read())
        .await
        .expect("Call failed");
    let mut buf = String::new();
    reader.read_to_string(&mut buf).await.unwrap();
    assert_eq!(buf, "content");
}

#[tokio::test]
async fn open_file_with_write_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "content").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write())
        .await
        .expect("Call failed");
    writer.write_all(b"CON").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "CONtent").await;
}

#[tokio::test]
async fn open_file_with_append_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "first").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().append())
        .await
        .expect("Call failed");
    writer.write_all(b"second").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "firstsecond").await;
}

#[tokio::test]
async fn open_file_with_truncate_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.sftp.fs().write(&path, "longfirstcontent").await.unwrap();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().truncate())
        .await
        .expect("Call failed");
    writer.write_all(b"second").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "second").await;
}

#[tokio::test]
async fn open_file_with_create_should_work() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    let mut writer = test_data
        .implementation
        .open_file(&path, &LinuxOpenOptions::new().write().create())
        .await
        .expect("Call failed");
    writer.write_all(b"content").await.unwrap();
    writer.flush().await.unwrap();
    test_data.assert_file(&path, "content").await;
}

#[tokio::test]
async fn create_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let path = gen_tmp_path();
    test_data.implementation.create_file(&path).await.expect("Call failed");
    test_data.assert_file_exists(&path, true).await;
}

#[tokio::test]
async fn rename_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    test_data.sftp.fs().write(&old_path, "new content").await.unwrap();
    test_data
        .implementation
        .rename_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file_exists(&old_path, false).await;
    test_data.assert_file(&new_path, "new content").await;
}

#[tokio::test]
async fn copy_file_should_persist() {
    let test_data = OpensshData::setup().await;
    let old_path = gen_tmp_path();
    let new_path = gen_tmp_path();
    test_data.sftp.fs().write(&old_path, "content").await.unwrap();
    test_data
        .implementation
        .copy_file(&old_path, &new_path)
        .await
        .expect("Call failed");
    test_data.assert_file(&old_path, "content").await;
    test_data.assert_file(&new_path, "content").await;
}
