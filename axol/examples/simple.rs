use std::borrow::Cow;

use axol::{
    grpc::{GrpcRequest, GrpcResponse},
    Logger, Path, Router, Server,
};
use log::info;

async fn index() -> &'static str {
    "hello world"
}

async fn var_page(Path(var): Path<Cow<'_, str>>) -> String {
    format!("hello {var}")
}

async fn grpc_health_check(request: GrpcRequest<()>) -> GrpcResponse<()> {
    info!("grpc = {request:?}");
    GrpcResponse::default()
}

fn route() -> Router {
    Router::new()
        .plugin("/", Logger::default())
        .get("/", index)
        .get("/:var", var_page)
        .post("/test.Test/HealthCheck", grpc_health_check)
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .parse_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let router = route();

    Server::bind("127.0.0.1:9081".parse().unwrap())
        .unwrap()
        .router(router)
        .serve()
        .await
        .unwrap();
}
