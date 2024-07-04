use common::TestSsh;

mod common;

#[tokio::test]
async fn t() {
    let test_ssh = TestSsh::setup().await;
}
