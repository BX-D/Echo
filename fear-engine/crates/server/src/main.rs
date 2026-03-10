//! # Fear Engine — Server
//!
//! The HTTP and WebSocket server for Fear Engine, built on Axum. Serves the game API,
//! manages WebSocket connections for real-time gameplay, and coordinates between the
//! game engine, fear profiling system, AI integration, and storage layers.

use std::sync::Arc;

use fear_engine_common::config::AppConfig;
use fear_engine_storage::Database;
use tokio::net::TcpListener;

pub mod app;
pub mod director;
pub mod game_loop;
pub mod middleware;
#[cfg(test)]
mod comprehensive_test;
pub mod session_script;
#[cfg(test)]
mod e2e_test;
#[cfg(test)]
mod perf_tests;
pub mod routes;
pub mod ws;

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();

    let db = Database::new(&config.database_url).expect("failed to initialize database");

    let state = Arc::new(app::AppState::new(db));

    let cors = middleware::cors::build_cors(&config);
    let app = app::build_app(state, cors);

    let addr = config.socket_addr();
    println!("Fear Engine server starting on {addr}");

    let listener = TcpListener::bind(&addr).await.expect("failed to bind to address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    println!("\nShutting down...");
}
