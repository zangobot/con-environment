use axum::{
    Router, response::{Html, IntoResponse, Response}, routing::get
};
use kube::Client;
use pingora::{proxy::http_proxy_service, server::Server};
use reqwest::StatusCode;
use tower_cookies::CookieManagerLayer;
use std::net::SocketAddr;
use std::sync::Arc;

// Project modules
mod auth;
mod config;
mod error;
mod gc;
mod orchestrator;
mod proxy;

pub use error::HubError;

use crate::gc::GarbageCollector;


pub static SIDECAR: &'static str = "ghcr.io/nbhdai/workshop-sidecar:latest";

/// Global application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    /// Client for talking to the Kubernetes API.
    kube_client: Client,
    config: Arc<config::Config>, 
}

impl AppState {
    pub fn gc(&self) -> GarbageCollector {
        GarbageCollector::new(self.kube_client.clone(), self.config.clone())
    }
}

async fn index() -> Result<Response, StatusCode> {
    return Ok(Html(include_str!("default_index.html")).into_response());
}

async fn pending_handler() -> Html<&'static str> {
    // This contains the <meta refresh> tag
    Html(include_str!("error_503.html")) 
}

async fn capacity_handler() -> Html<&'static str> {
    Html("<h1>Workshop at Capacity</h1><p>Please try again later.</p>")
}

async fn staff_error_handler() -> Html<&'static str> {
    Html("<h1>Setup Failed</h1><p>Please contact staff for assistance.</p>")
}

#[tokio::main]
async fn main() {
    // Initialize logging, config, kube client...
    tracing_subscriber::fmt::init();
    let config = Arc::new(config::Config::from_env().expect("Failed to load config from env"));
    let kube_client = Client::try_default().await.expect("Kube client failed");

    tracing::info!("Config loaded: {:?}", config);
    let state = AppState {
        kube_client: kube_client.clone(),
        config: config.clone(),
    };
    
    let axum_app = Router::new()
        .merge(auth::auth_routes())
        .route("/", get(index).post(auth::handle_login))
        .route("/workshop-pending", get(pending_handler))
        .route("/workshop-at-capacity", get(capacity_handler))
        .route("/workshop-error", get(staff_error_handler))
        .with_state(state)
        .layer(CookieManagerLayer::new());

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        tracing::info!("Axum (Internal UI) listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, axum_app).await.unwrap();
    });

    let mut my_server = Server::new(None).unwrap();
    my_server.bootstrap();

    let proxy_logic = proxy::WorkshopProxy {
        kube_client: kube_client.clone(),
        config: config.clone(),
    };

    let mut lb = http_proxy_service(
        &my_server.configuration,
        proxy_logic
    );
    
    // Bind to the PUBLIC port
    lb.add_tcp("0.0.0.0:8080"); 

    my_server.add_service(lb);
    
    tracing::info!("Pingora Gateway listening on 0.0.0.0:8080");
    my_server.run_forever();
}

#[cfg(test)]
mod tests {
    pub mod gc;
    pub mod helpers;
    pub mod config;
    pub mod integration;
}
