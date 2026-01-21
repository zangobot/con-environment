use crate::{auth, HubError};
use async_trait::async_trait;
use axum::http::Uri;
use pingora::prelude::*;

pub struct WorkshopProxy;

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
        let orchestrator = crate::orchestrator().await;
        let cookie_header = session
            .req_header()
            .headers
            .get("Cookie")
            .map(|v| v.to_str().unwrap_or_default())
            .unwrap_or_default();
        let local_path = if let Some(user) = auth::validate_cookie(cookie_header) {
            match orchestrator.get_or_create_pod(&user.user_id).await {
                Ok(url) => {
                    let peer = Box::new(HttpPeer::new(
                        url,
                        false,
                        String::new(),
                    ));
                    return Ok(peer);
                }
                Err(HubError::PodLimitReached) => "/aiv-workshop-at-capacity",
                Err(HubError::PodNotReady) => "/aiv-workshop-pending",
                Err(HubError::KubeError{ operation, source }) => {
                    tracing::error!("Unhandled orchestrator error for {}, during: {}: {:?}", user.user_id, operation, source);
                    "/aiv-workshop-error"
                }
            }
        } else {
            "/aiv-workshop-login"
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
