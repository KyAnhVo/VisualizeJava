use std::env;

use axum::{Router, http::Method, routing::post};
use tower_http::cors::{Any, CorsLayer};

use crate::server::handlers::graph_construct_handler;

pub mod abstraction_graph;
pub mod name_resolution;
pub mod parser;
pub mod resolved_types;
pub mod server;
pub mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app: Router = Router::new()
        .route("/graph", post(graph_construct_handler))
        .layer(cors);
    let listener =
        tokio::net::TcpListener::bind("0.0.0.0:".to_string() + env::var("PORT").unwrap().as_str())
            .await
            .unwrap();
    axum::serve(listener, app).await.unwrap();
}
