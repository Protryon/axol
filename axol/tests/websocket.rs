use axol::{Message, Router, WebSocketUpgrade};
use axol_http::response::Response;

mod common;
use common::*;
use futures::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message as TTMessage;

async fn simple_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut c| async move {
        match c.recv().await.unwrap().unwrap() {
            Message::Text(text) => {
                c.send(Message::Text(text)).await.unwrap();
            }
            _ => panic!("invalid message at server"),
        }
    })
}

#[tokio::test]
async fn websocket_tests() {
    let handle = spawn_router(Router::new().get("/ws", simple_ws)).await;

    let (mut stream, _) = tokio_tungstenite::connect_async(format!("ws://{}/ws", *TEST_ADDRESS))
        .await
        .unwrap();

    stream
        .send(TTMessage::Text(format!("test message")))
        .await
        .unwrap();
    match stream.next().await.unwrap().unwrap() {
        TTMessage::Text(text) => {
            assert_eq!(text, "test message");
        }
        _ => panic!("invalid message at client"),
    }

    handle.abort();
}
