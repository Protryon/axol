use std::borrow::Cow;

use axol::{Logger, Path, Router, Server};

async fn index() -> &'static str {
    "hello world"
}

async fn var_page(Path(var): Path<Cow<'_, str>>) -> String {
    format!("hello {var}")
}

fn route() -> Router {
    Router::new()
        .plugin("/", Logger::default())
        .get("/", index)
        .get("/:var", var_page)
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
