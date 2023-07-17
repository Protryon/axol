use axol::{Logger, Path, Router, Server};

async fn index() -> &'static str {
    "hello world"
}

async fn var_page(Path(var): Path<String>) -> String {
    format!("hello {var}")
}

fn route() -> Router {
    Router::new()
        .get("/", index)
        .get("/:var", var_page)
        .plugin("/", Logger::default())
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
        .build()
        .unwrap()
        .serve()
        .await
        .unwrap();
}
