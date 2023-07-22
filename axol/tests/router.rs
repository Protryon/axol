use std::borrow::Cow;

use axol::{Path, Query, Router};
use axol_http::StatusCode;
use serde::Deserialize;

mod common;
use common::*;

async fn simple_get() -> &'static str {
    "success"
}

async fn empty_get() {}

async fn simple_path(Path(path): Path<Cow<'_, str>>) -> String {
    format!("success {path}")
}

#[derive(Deserialize)]
struct SimpleQuery<'a> {
    name: Cow<'a, str>,
}

async fn simple_query(Query(SimpleQuery { name }): Query<SimpleQuery<'_>>) -> String {
    format!("success {name}")
}

#[tokio::test]
async fn router_tests() {
    let handle = spawn_router(
        Router::new()
            .get("/", simple_get)
            .get("/empty", empty_get)
            .get("/var/:var", simple_path)
            .get("/query", simple_query),
    )
    .await;

    let response = reqwest::get(format!("http://{}/", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());
    assert_eq!(&response.bytes().await.unwrap(), &b"success"[..]);

    let response = reqwest::get(format!("http://{}/empty", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());

    let response = reqwest::get(format!("http://{}/fake", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::NotFound, response.status().into());

    let response = reqwest::get(format!("http://{}/var/test", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());
    assert_eq!(&response.bytes().await.unwrap(), &b"success test"[..]);

    let response = reqwest::get(format!("http://{}/query?name=west", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());
    assert_eq!(&response.bytes().await.unwrap(), &b"success west"[..]);

    let response = reqwest::get(format!("http://{}/query/?name=east", *TEST_ADDRESS))
        .await
        .unwrap();
    assert_eq!(StatusCode::Ok, response.status().into());
    assert_eq!(&response.bytes().await.unwrap(), &b"success east"[..]);

    handle.abort();
}
