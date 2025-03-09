mod config;
mod services;

use crate::config::Config;
use crate::services::db_service::DbService;
use crate::services::docker_service::DockerService;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tower_http::ServiceBuilderExt;

#[derive(Clone)]
struct AppState {
    config: Config,
    db_service: DbService,
    docker_service: DockerService,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(15)))
                .compression(),
        )
        .with_state(AppState {
            config: Config::parse().await,
            db_service: DbService::new(),
            docker_service: DockerService::new(),
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
