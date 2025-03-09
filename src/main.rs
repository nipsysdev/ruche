mod models;

use std::net::SocketAddr;
use axum::{Router, routing::get};
use crate::models::config::{parse_config, Config};

#[derive(Clone)]
struct AppState {
    config: Config
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .with_state(AppState { config: parse_config().await });
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
