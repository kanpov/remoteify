use common::OpensshData;

mod common;

#[tokio::test]
async fn t() {
    let test_data = OpensshData::setup().await;
}
