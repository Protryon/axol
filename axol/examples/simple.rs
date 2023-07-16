use axol::{Router, Server};

async fn index() -> &'static str {
    "hello world"
}

fn route() -> Router {
    Router::new().get("/", index)
}

#[tokio::main]
async fn main() {
    Server::bind("127.0.0.1:9081".parse().unwrap())
        .unwrap()
        .router(route())
        .build()
        .unwrap()
        .serve()
        .await
        .unwrap();
}
