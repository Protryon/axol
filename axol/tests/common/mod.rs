use std::{net::SocketAddr, time::Duration};

use axol::Router;
use tokio::task::JoinHandle;

lazy_static::lazy_static! {
    pub static ref TEST_ADDRESS: SocketAddr = "127.0.0.1:9801".parse().unwrap();
}

pub async fn run_router(router: Router) {
    axol::Server::bind(*TEST_ADDRESS)
        .expect("bind failure")
        .router(router)
        .serve()
        .await
        .expect("server failed");
    std::process::exit(1);
}

pub async fn spawn_router(router: Router) -> JoinHandle<()> {
    let out = tokio::spawn(async move { run_router(router).await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    out
}
