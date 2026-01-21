use kube::Error as KubeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HubError {
    #[error("Kubernetes API error during {operation}: {source}")]
    KubeError {
        operation: &'static str,
        #[source]
        source: KubeError,
    },

    #[error("Pod failed to become ready in time")]
    PodNotReady,

    #[error("Global pod limit reached")]
    PodLimitReached,
}

// We can implement IntoResponse for our error
// (Not fully done here, but shows the idea)
impl axum::response::IntoResponse for HubError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            HubError::KubeError { .. } => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            HubError::PodNotReady => (
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                "Workshop failed to start".to_string(),
            ),
            HubError::PodLimitReached => (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "Service is at capacity, please try again later".to_string(),
            ),
        };
        (status, message).into_response()
    }
}
