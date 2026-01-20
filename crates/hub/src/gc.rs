use std::sync::Arc;

use chrono::Utc;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{Client, api::{Api, DeleteParams, ListParams}};
use serde::Deserialize;
use tracing::{info, warn};

use crate::config::{Config, LABEL_WORKSHOP_NAME, TTL_ANNOTATION};

pub struct GarbageCollector {
    pod_api: Api<Pod>,
    svc_api: Api<Service>, // <-- Added: Accept Service API
    config: Arc<Config>,
    max_idle_seconds: i64,
}

#[derive(Deserialize, Debug)]
struct SidecarHealth {
    status: String,
    last_activity_timestamp: i64,
    idle_seconds: i64,
}
impl GarbageCollector {
    pub fn new(client: Client, config: Arc<Config>) -> Self {
        let pod_api: Api<Pod> = Api::namespaced(
            client.clone(),
            &config.workshop_namespace
        );
        let svc_api = kube::Api::<Service>::namespaced(
            client.clone(),
            &config.workshop_namespace,
        );
        Self {
            max_idle_seconds: config.workshop_idle_seconds,
            pod_api,
            svc_api,
            config,
        }
    }

    #[cfg(test)]
    pub(crate) fn override_max_idle(mut self, max_idle_seconds: i64) -> Self {
        self.max_idle_seconds = max_idle_seconds;
        self
    }

/// Iterates through all managed pods and cleans up idle ones.
pub async fn cleanup_idle_pods(&self) -> Result<(), crate::HubError> {
    let list_params = ListParams::default().labels(&format!(
        "{}={},{}={}",
        "app.kubernetes.io/managed-by", "workshop-hub", LABEL_WORKSHOP_NAME, self.config.workshop_name
    ));

    let pods = self.pod_api.list(&list_params).await?;
    let client = reqwest::Client::new();

    if pods.items.is_empty() {
        info!("GC: No managed pods found.");
        return Ok(());
    }

    info!("GC: Checking {} managed pods...", pods.items.len());

    // Extract namespace from the Api - this is what the Api is namespaced to
    let namespace = &self.config.workshop_namespace;

    let now = Utc::now().timestamp();

    for pod in pods.items {
        let pod_name = pod.metadata.name.as_deref().unwrap_or_default();
        if pod_name.is_empty() {
            continue;
        }

        // The service name is assumed to match the pod name
        let service_name = pod_name;
        let delete_resources = || async {
            let dp = DeleteParams::default();
            if let Err(e) = self.svc_api.delete(service_name, &dp).await {
                warn!("GC: Failed to delete service {} (may vary based on OwnerRef): {}", service_name, e);
            } else {
                info!("GC: Deleted service {}", service_name);
            }
            
            // Delete the pod
            self.pod_api.delete(pod_name, &dp).await
        };

        // --- TTL Check ---
        // Check for TTL expiration first
        if let Some(annotations) = &pod.metadata.annotations {
            if let Some(expires_at_str) = annotations.get(TTL_ANNOTATION) {
                if let Ok(expires_at) = expires_at_str.parse::<i64>() {
                    if now > expires_at {
                        info!("GC: Pod {} has exceeded its max TTL. Deleting.", pod_name);
                        delete_resources().await?;
                        continue; // Move to next pod
                    }
                }
            }
        }

        // --- State Check ---
        // Pods in Pending/Failed/Succeeded state should be checked
        let phase = pod.status.as_ref().and_then(|s| s.phase.as_deref());
        if phase != Some("Running") {
            warn!("GC: Found non-Running pod {}. Deleting.", pod_name);
            delete_resources().await?;
            continue;
        }

        // Pod is running, check its health endpoint
        // Connect to the service's "health" port using the namespace from the Api
        let health_url = format!(
            "http://{}.{}.svc.cluster.local:9000/health",
            service_name, namespace
        );

        match client
            .get(&health_url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    warn!(
                        "GC: Health check for {} failed (status: {}). Deleting.",
                        pod_name,
                        response.status()
                    );
                    delete_resources().await?;
                    continue;
                }

                match response.json::<SidecarHealth>().await {
                    Ok(health) => {
                        info!("GC: Pod {} idle for {}s", pod_name, health.idle_seconds);
                        if health.idle_seconds > self.max_idle_seconds {
                            info!("GC: Pod {} exceeded idle time. Deleting.", pod_name);
                            delete_resources().await?;
                        }
                    }
                    Err(e) => {
                        warn!(
                            "GC: Failed to parse health from {}: {}. Deleting.",
                            pod_name, e
                        );
                        delete_resources().await?;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "GC: Health check request for {} failed: {}. Deleting.",
                    pod_name, e
                );
                delete_resources().await?;
            }
        }
    }

    Ok(())
}
}