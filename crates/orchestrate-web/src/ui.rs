//! React SPA UI routes for the web interface

use axum::Router;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

use crate::api::AppState;

/// Create the UI router that serves the React SPA
pub fn create_ui_router() -> Router<Arc<AppState>> {
    let static_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    let assets_path = static_path.join("assets");
    let index_path = static_path.join("index.html");

    Router::new()
        // Serve static assets (JS, CSS, images) from /assets
        .nest_service("/assets", ServeDir::new(assets_path))
        // Serve favicon
        .nest_service(
            "/favicon.svg",
            ServeFile::new(static_path.join("favicon.svg")),
        )
        // SPA fallback - all other routes serve index.html for client-side routing
        .fallback_service(ServeFile::new(index_path).precompressed_gzip())
}
