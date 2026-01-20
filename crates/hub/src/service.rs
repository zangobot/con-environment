use async_trait::async_trait;
use axum::{
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use pingora::server::Fds;
use reqwest::StatusCode;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    signal,
    sync::{watch::Receiver, Mutex},
};
use tower_cookies::CookieManagerLayer;

use crate::auth;

pub struct AxumService {
    pub router: Router,
    pub addr: SocketAddr,
}

impl AxumService {
    pub fn new() -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        let router = Router::new()
            .merge(auth::auth_routes())
            .route("/", get(index).post(auth::handle_login))
            .route("/workshop-pending", get(pending_handler))
            .route("/workshop-at-capacity", get(capacity_handler))
            .route("/workshop-error", get(staff_error_handler))
            .layer(CookieManagerLayer::new());
        Self { router, addr }
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

// 2. Implement the Pingora Service trait
#[async_trait]
impl pingora::services::Service for AxumService {
    async fn start_service(
        &mut self,
        _fds: Option<Arc<Mutex<Fds>>>,
        mut shutdown: Receiver<bool>,
        _listeners_per_fd: usize,
    ) {
        tracing::info!("Starting Axum Service on {}", self.addr);
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .expect("Failed to bind Axum port");
        let shutdown_fn = || async move {
            let ctrl_c = async {
                signal::ctrl_c()
                    .await
                    .expect("failed to install Ctrl+C handler");
            };

            #[cfg(unix)]
            let terminate = async {
                signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("failed to install signal handler")
                    .recv()
                    .await;
            };

            let pingora_signal = shutdown.changed();

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
                _ = pingora_signal => {},
            }
        };

        if let Err(e) = axum::serve(listener, self.router.clone())
            .with_graceful_shutdown(shutdown_fn())
            .await
        {
            tracing::error!("Axum server error: {}", e);
        }
    }

    fn name(&self) -> &str {
        "Axum Internal UI"
    }
}
