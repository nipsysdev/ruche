mod config;
mod constants;
mod handlers;
mod models;
mod services;
mod utils;

use crate::config::Config;
use crate::handlers::bees_handler::init_bees_handler;
use crate::services::bee_service::BeeService;
use crate::services::db_service::DbService;
use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tower_http::ServiceBuilderExt;

#[derive(Clone)]
pub struct AppState {
    bee_service: BeeService,
    last_bee_deletion_req: Arc<Mutex<HashMap<u8, SystemTime>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse().await;
    let db_service = DbService::new();
    // let docker_service = DockerService::new();

    let app_state: Arc<AppState> = Arc::new(AppState {
        bee_service: BeeService::new(config, Box::new(db_service)),
        last_bee_deletion_req: Arc::new(Mutex::new(HashMap::new())),
    });

    let app = Router::new()
        .nest("/bees", init_bees_handler(app_state.clone()))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::new(Duration::from_secs(15)))
                .compression(),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
