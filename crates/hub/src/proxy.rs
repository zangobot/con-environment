use crate::{auth, config::Config, orchestrator, HubError};
use async_trait::async_trait;
use axum::http::Uri;
use pingora::prelude::*;
use std::sync::Arc;

pub struct WorkshopProxy {
    pub config: Arc<Config>,
}

#[async_trait]
impl ProxyHttp for WorkshopProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {
        ()
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let cookie_header = session
            .req_header()
            .headers
            .get("Cookie")
            .map(|v| v.to_str().unwrap_or_default())
            .unwrap_or_default();
        let local_path = if let Some(user) = auth::validate_cookie(cookie_header) {
            match orchestrator::get_or_create_pod(&user.user_id, self.config.clone()).await {
                Ok(binding) => {
                    let peer = Box::new(HttpPeer::new(
                        binding.cluster_dns_name,
                        false,
                        String::new(),
                    ));
                    return Ok(peer);
                }
                Err(HubError::PodLimitReached) => "/aiv-workshop-at-capacity",
                Err(HubError::PodNotReady) => "/aiv-workshop-pending",
                Err(HubError::KubeError(e)) => {
                    tracing::error!("Unhandled orchestrator error for {}: {:?}", user.user_id, e);
                    "/aiv-workshop-error"
                }
            }
        } else {
            "/login"
        };

        session
            .req_header_mut()
            .set_uri(Uri::from_static(local_path));

        // 5. PROXY TO LOCAL AXUM
        // Instead of failing, we successfully proxy to our internal UI
        let peer = Box::new(HttpPeer::new("127.0.0.1:3000", false, String::new()));
        Ok(peer)
    }
}
