//! # Echo Protocol — Server
//!
//! The HTTP and WebSocket server for Echo Protocol, built on Axum. Serves the game API,
//! manages WebSocket connections for real-time gameplay, and coordinates the
//! script-driven runtime, prompt integration, and storage layers.

use std::sync::Arc;

use fear_engine_common::config::AppConfig;
use fear_engine_storage::Database;
use tokio::net::TcpListener;

pub mod app;
pub mod director;
pub mod echo_protocol;
pub mod game_loop;
pub mod middleware;
pub mod routes;
pub mod session_script;
pub mod ws;

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();

    let db = Database::new(&config.database_url).expect("failed to initialize database");

    let state = Arc::new(app::AppState::new(db));

    let cors = middleware::cors::build_cors(&config);
    let app = app::build_app(state, cors);

    let addr = config.socket_addr();
    println!("Echo Protocol server starting on {addr}");

    let listener = TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address");

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
