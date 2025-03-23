mod bee_service;
mod core;
mod handlers;
mod models;
mod utils;

use crate::core::database::Database;
use crate::handlers::bee_handlers::init_bee_handlers;
use axum::Router;
use bee_service::BeeService;
use core::docker::Docker;
use handlers::bees_handlers::init_bees_handlers;
use models::config::Config;
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
    let database = Database::new();
    let docker = Docker::new();

    let app_state: Arc<AppState> = Arc::new(AppState {
        bee_service: BeeService::new(config, Box::new(database), Box::new(docker)),
        last_bee_deletion_req: Arc::new(Mutex::new(HashMap::new())),
    });

    let app = Router::new()
        .nest("/bee", init_bee_handlers(app_state.clone()))
        .nest("/bees", init_bees_handlers(app_state.clone()))
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
