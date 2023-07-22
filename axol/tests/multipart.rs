use axol::{Multipart, Router};
use axol_http::StatusCode;

mod common;
use common::*;
use reqwest::multipart::Part;

async fn simple_multipart(mut mp: Multipart) {
    let field = mp.next_field().await.unwrap().unwrap();
    assert_eq!(field.file_name(), Some("test.txt"));
    assert_eq!(&field.bytes().await.unwrap(), &b"test message"[..]);
}

#[tokio::test]
async fn multipart_tests() {
    let handle = spawn_router(Router::new().post("/mp", simple_multipart)).await;

    let form = reqwest::multipart::Form::new().part(
        "test file",
        Part::bytes(&b"test message"[..]).file_name("test.txt"),
    );

    let response = reqwest::Client::new()
        .post(format!("http://{}/mp", *TEST_ADDRESS))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());

    handle.abort();
}
