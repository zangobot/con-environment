use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Utc;
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    Api, ResourceExt, Error as KubeError, Client as KubeClient
};
use k8s_openapi::api::core::v1::{Pod, Service};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use papaya::{Guard, HashMap};
use serde_json::json;
use tracing::{info, warn};

use crate::{config, HubError};

mod pod;
// Re-export ManagedPod and Health structs
pub use pod::{ManagedPod, SidecarHealth};

// Constants
const LABEL_USER_ID: &str = "workshop-hub/user-id";
const LABEL_MANAGED_BY: &str = "app.kubernetes.io/managed-by";
const HUB_ID: &str = "workshop-hub";
pub const TTL_ANNOTATION: &str = "workshop-hub/ttl-expires-at";
pub const LABEL_WORKSHOP_NAME: &str = "workshop-hub/workshop-name";

pub struct Orchestrator {
    pub config: config::Config,
    pub pods: HashMap<String, ManagedPod>,
    http_client: reqwest::Client,
    pod_api: Api<Pod>,
    svc_api: Api<Service>,
}

pub enum PodStatus {
    Healthy(Pod, String),
    Old(ManagedPod),
    Unhealthy(ManagedPod),
    PodMissing,
    Missing,
    UnreachableError,
}

impl Orchestrator {
    pub async fn new() -> Self {
        let config = config::Config::from_env();
        Self::with_config(config).await
    }

    pub async fn with_config(config: config::Config) -> Self {
        let pods = HashMap::new();
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Failed to create HTTP client");
        let kube_client = KubeClient::try_default().await.expect("Kube client failed");
        let pod_api: Api<Pod> = Api::namespaced(kube_client.clone(), &config.workshop_namespace);
        let svc_api: Api<Service> = Api::namespaced(kube_client.clone(), &config.workshop_namespace);
        Self {
            config,
            pods,
            http_client,
            pod_api,
            svc_api
        }
    }

    pub fn guard(&self) -> impl Guard + '_ {
        self.pods.owned_guard()
    }

    /// 1. POPULATE: Syncs in-memory state with Kubernetes.
    /// Lists all pods in the namespace and ensures they exist in the HashMap.
    pub async fn populate(&self) -> Result<(), HubError> {
        let list_params = ListParams::default().labels(&format!(
            "{}={},{}={}",
            LABEL_MANAGED_BY, HUB_ID,
            LABEL_WORKSHOP_NAME, &self.config.workshop_name
        ));

        let k_pods = self.pod_api.list(&list_params).await.map_err(|source| {
            HubError::KubeError { operation: "populate", source }
        })?;
        
        // Lockless iteration over K8s results
        for k_pod in k_pods.items {
            if let Some(user_id) = k_pod.metadata.labels.as_ref().and_then(|l| l.get(LABEL_USER_ID)) {
                let user_id = user_id.clone();
                
                // Update existing or insert new (Adoption)
                self.pods.pin().update_or_insert_with(
                    user_id.clone(),
                    |mp| {
                        let mut mp = mp.clone();
                        mp.set_pod(k_pod.clone());
                        mp
                    },
                    || {
                        let mut mp = ManagedPod::default();
                        mp.set_pod(k_pod.clone());
                        mp
                    },
                );
            }
        }
        Ok(())
    }

    pub async fn check_health<'guard>(
        &self,
        user_id: &str,
        guard: &'guard impl Guard,
    ) -> Result<PodStatus, KubeError> {
        // 1. Look up managed pod
        let mp = match self.pods.get(user_id, guard) {
            Some(mp) => mp.clone(),
            None => return Ok(PodStatus::Missing),
        };

        // 2. Fast path: recently verified healthy
        if let Some(status) = self.check_cached_health(&mp) {
            return Ok(status);
        }

        // 3. Ensure we have a pod (recover from K8s if needed)
        let mp = match self.ensure_pod_loaded(user_id, &mp, guard).await? {
            Some(mp) => mp,
            None => return Ok(PodStatus::Missing),
        };

        // 4. Perform actual health check
        Ok(self.perform_health_check(user_id, &mp, guard).await)
    }

    /// Fast path: if we checked recently and pod has an IP, skip the health call
    fn check_cached_health(&self, mp: &ManagedPod) -> Option<PodStatus> {
        const FRESHNESS_DIVISOR: i64 = 4;
        let freshness_threshold = self.config.workshop_idle_seconds / FRESHNESS_DIVISOR;

        let pod = mp.pod()?;
        if mp.idle() >= freshness_threshold {
            return None; // Cache is stale, need fresh check
        }

        let pod_ip = pod.status.as_ref()?.pod_ip.as_ref()?;
        let url = self.build_proxy_url(pod_ip);
        Some(PodStatus::Healthy(pod.clone(), url))
    }

    /// If ManagedPod has no pod data, try to recover it from K8s
    async fn ensure_pod_loaded<'guard>(
        &self,
        user_id: &str,
        mp: &'guard ManagedPod,
        guard: &'guard impl Guard,
    ) -> Result<Option<&'guard ManagedPod>, KubeError> {
        // Already have pod data
        if mp.pod().is_some() {
            return Ok(Some(mp));
        }

        // Try to recover from K8s
        let list_params = ListParams::default().labels(&format!(
            "{}={},{}={},{}={}",
            LABEL_USER_ID, user_id,
            LABEL_WORKSHOP_NAME, &self.config.workshop_name,
            LABEL_MANAGED_BY, HUB_ID
        ));

        let k_pod = match self.pod_api.list(&list_params).await?.items.pop() {
            Some(pod) => pod,
            None => {
                warn!("Pod for user {} in memory but missing in K8s", user_id);
                return Ok(None);
            }
        };

        info!("Recovered existing K8s pod for user {}", user_id);
        // This returns the updated value
        let updated = self.pods.update(
            user_id.to_string(),
            |mp| {
                let mut mp = mp.clone();
                mp.set_pod(k_pod.clone());
                mp
            },
            guard,
        );

        Ok(updated)
    }

    /// Actually call the sidecar health endpoint and update state
    async fn perform_health_check<'guard>(
        &self,
        user_id: &str,
        mp: &ManagedPod,
        guard: &'guard impl Guard,
    ) -> PodStatus {
        let pod = match mp.pod() {
            Some(p) => p,
            None => return PodStatus::Missing,
        };

        let (health, pod_ip) = match self.query_sidecar_health(user_id, pod).await {
            Some(result) => result,
            None => return PodStatus::Unhealthy(mp.clone()),
        };

        // Update cached health state
        let mut updated_mp = mp.clone();
        updated_mp.set_health(health);
        self.pods.insert(user_id.to_string(), updated_mp.clone(), guard);

        // Check if pod has exceeded idle timeout
        if updated_mp.idle() >= self.config.workshop_idle_seconds {
            PodStatus::Old(updated_mp)
        } else {
            let url = self.build_proxy_url(&pod_ip);
            PodStatus::Healthy(pod.clone(), url)
        }
    }

    /// Query the sidecar's health endpoint
    async fn query_sidecar_health(
        &self,
        user_id: &str,
        pod: &Pod,
    ) -> Option<(SidecarHealth, String)> {
        let pod_ip = pod.status.as_ref()?.pod_ip.as_ref()?;
        let url = format!(
            "http://{}:{}/health",
            pod_ip, self.config.sidecar_health_port
        );

        let resp = match self.http_client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                warn!("Sidecar unreachable for {} ({}): {}", user_id, url, e);
                return None;
            }
        };

        if !resp.status().is_success() {
            warn!("Health check returned {} for {}", resp.status(), user_id);
            return None;
        }

        match resp.json::<SidecarHealth>().await {
            Ok(health) => Some((health, pod_ip.clone())),
            Err(e) => {
                warn!("Failed to parse health JSON for {}: {}", user_id, e);
                None
            }
        }
    }

    /// Build the proxy URL from a pod IP
    fn build_proxy_url(&self, pod_ip: &str) -> String {
        format!("http://{}:{}", pod_ip, self.config.sidecar_proxy_port)
    }

    /// 3. DELETE: Removes K8s resources and clears the map entry.
    pub async fn delete(&self, user_id: &str) -> Result<(), HubError> {
        // Get pod name from map first
        let pod_name = {
            let guard = self.pods.pin();
            match guard.get(user_id) {
                Some(mp) => mp.pod().map(|p| p.name_any()),
                None => return Ok(()), // Already gone
            }
        };

        if let Some(name) = pod_name {
            info!("Orchestrator: Deleting resources for user {}", user_id);
            let dp = DeleteParams::default();
            
            // Delete Service (ignore 404s)
            let _ = self.svc_api.delete(&name, &dp).await;
            
            // Delete Pod
            let _ = self.pod_api.delete(&name, &dp).await;
        }

        // Remove from memory
        self.pods.pin().remove(user_id);
        
        Ok(())
    }

    /// Retrieves an existing pod, recovers state from K8s if missing, or creates a new one.
    pub async fn get_or_create_pod(&self, user_id: &str) -> Result<String, HubError> {
        // 1. FAST PATH: Check In-Memory Map
        // We use a separate scope to ensure the guard is dropped before awaiting any async calls.
        let guard = self.guard();
        match {
            self.check_health(user_id, &guard).await.map_err(|source| {
                HubError::KubeError { operation: "get_or_create_pod", source }
            })?
        } {
            PodStatus::Healthy(_ , url) => return Ok(url),
            PodStatus::Old(mp) => {
                // Old pod that wasn't cleaned up yet
                match mp.pod() {
                    Some(pod) => {
                        match pod.status.as_ref().and_then(|s| s.pod_ip.as_ref()) {
                            Some(pod_ip) => {
                                let url = format!(
                                    "http://{}:{}",
                                    pod_ip, self.config.sidecar_proxy_port
                                );
                                return Ok(url);
                            },
                            None => {
                                // Pod exists in k8s/memory but hasn't been assigned an IP yet.
                                // It is likely in a Pending state.
                                self.pods.remove(user_id, &guard);
                            }
                        }
                    },
                    None => {
                        self.pods.remove(user_id, &guard);
                    },
                }
            }
            PodStatus::Unhealthy(_) | PodStatus::UnreachableError => return Err(HubError::PodNotReady),
            PodStatus::Missing | PodStatus::PodMissing => {
                // We need to make the pod
                self.pods.remove(user_id, &guard);
            },
        }

        // 2. SLOW PATH: Create New Pod
        // B. Check Global Limits
        if self.pods.len() >= self.config.workshop_pod_limit {
            return Err(HubError::PodLimitReached);
        }

        // C. Create Resources
        let pod_name = format!("workshop-{}-{}", user_id, generate_suffix());
        let expires_at = Utc::now().timestamp() + self.config.workshop_ttl_seconds;

        let pod_spec = create_workshop_pod_spec(&pod_name, user_id, &self.config, expires_at);
        let pod = self.pod_api.create(&PostParams::default(), &pod_spec).await.map_err(|source| {
                HubError::KubeError { operation: "get_or_create_pod", source }
            })?;
        info!("Created pod {}", pod_name);

        let owner_ref = OwnerReference {
            api_version: "v1".to_string(),
            kind: "Pod".to_string(),
            name: pod_name.clone(),
            uid: pod.metadata.uid.clone().unwrap_or_default(),
            ..Default::default()
        };

        let svc_spec = create_workshop_service_spec(&pod_name, &pod_name, user_id, &self.config.workshop_name, owner_ref, &self.config);
        self.svc_api.create(&PostParams::default(), &svc_spec).await.map_err(|source| {
                HubError::KubeError { operation: "get_or_create_pod", source }
            })?;

        self.pods.insert(user_id.to_string(), ManagedPod::default(), &guard);
        Err(HubError::PodNotReady)
    }

/// 4. GARBAGE COLLECTION: Removes expired or idle pods.
pub async fn gc(&self) -> Result<usize, HubError> {
    let mut candidates = Vec::new();
    let now = Utc::now().timestamp();
        let guard = self.pods.owned_guard();

    // 1. SCAN: Identify candidates (Fast, Lock-free read)
    for (user_id, mp) in self.pods.iter(&guard) {
        let is_idle = mp.idle() > self.config.workshop_idle_seconds;

        let is_expired = mp
            .pod()
            .and_then(|pod| pod.metadata.annotations.as_ref())
            .and_then(|ann| ann.get(TTL_ANNOTATION))
            .and_then(|t| t.parse::<i64>().ok())
            .map(|expires_at| now > expires_at)
            .unwrap_or(false);

        if is_idle || is_expired {
            candidates.push((user_id.clone(), is_expired));
        }
    }

    // 2. VERIFY & PURGE: Health check before deletion
    let mut deleted_count = 0;

    for (user_id, is_expired) in candidates {
        // Expired pods get deleted regardless of health
        if !is_expired {
            // For idle pods, do a fresh health check - an active connection
            // may have kept the pod alive without updating our cached state
            match self.check_health(&user_id, &guard).await {
                Ok(PodStatus::Healthy(_, _)) => {
                    info!(
                        "GC: Skipping {} - health check shows active connection",
                        user_id
                    );
                    continue;
                }
                Ok(PodStatus::Old(_)) => {
                    // Still old after fresh check, proceed with deletion
                }
                Ok(PodStatus::Unhealthy(_)) => {
                    // Unhealthy pods should be cleaned up
                }
                Ok(PodStatus::Missing | PodStatus::PodMissing) => {
                    // Already gone, just clean up map entry
                }
                Ok(PodStatus::UnreachableError) | Err(_) => {
                    // Can't reach pod - skip for now, try again next cycle
                    warn!(
                        "GC: Skipping {} - couldn't verify health, will retry",
                        user_id
                    );
                    continue;
                }
            }
        }

        info!(
            "GC: Deleting {} (expired: {})",
            user_id, is_expired
        );

        if let Err(e) = self.delete(&user_id).await {
            tracing::error!("GC: Failed to delete session for {}: {}", user_id, e);
        } else {
            deleted_count += 1;
        }
    }

    if deleted_count > 0 {
        info!("GC: Completed. Cleaned up {} sessions.", deleted_count);
    }

    Ok(deleted_count)
}
}


// --- Helpers ---

fn generate_suffix() -> String {
    use rand::Rng;
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(6)
        .map(char::from)
        .collect::<String>()
        .to_lowercase()
}

fn create_workshop_pod_spec(
    pod_name: &str,
    user_id: &str,
    config: &config::Config,
    expires_at_timestamp: i64,
) -> Pod {
    let mut labels = BTreeMap::new();
    labels.insert(LABEL_USER_ID.to_string(), user_id.to_string());
    labels.insert(LABEL_WORKSHOP_NAME.to_string(), config.workshop_name.clone());
    labels.insert(LABEL_MANAGED_BY.to_string(), HUB_ID.to_string());
    labels.insert("app".to_string(), pod_name.to_string());

    let mut annotations = BTreeMap::new();
    annotations.insert(TTL_ANNOTATION.to_string(), expires_at_timestamp.to_string());

    serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": pod_name,
            "labels": labels,
            "annotations": annotations
        },
        "spec": {
            "restartPolicy": "Never",
            "containers": [
                {
                    "name": "workshop",
                    "image": config.workshop_image,
                    "imagePullPolicy": "Always",
                    "ports": [{"containerPort": config.workshop_port}],
                    "resources": {
                        "requests": {
                            "cpu": config.workshop_cpu_request,
                            "memory": config.workshop_mem_request
                        },
                        "limits": {
                            "cpu": config.workshop_cpu_limit,
                            "memory": config.workshop_mem_limit
                        }
                    }
                },
                {
                    "name": "sidecar",
                    "image": crate::SIDECAR, // Ensure this constant is available or pass from config
                    "imagePullPolicy": "Always",
                    "env": [
                        {"name": "SIDECAR_HTTP_LISTEN", "value": format!("0.0.0.0:{}", config.sidecar_health_port)},
                        {"name": "SIDECAR_TCP_LISTEN", "value": format!("0.0.0.0:{}", config.sidecar_proxy_port)},
                        {"name": "SIDECAR_TARGET_TCP", "value": format!("127.0.0.1:{}", config.workshop_port)},
                    ],
                    "ports": [
                        {"name": "health", "containerPort": config.sidecar_health_port},
                        {"name": "proxy", "containerPort": config.sidecar_proxy_port}
                    ],
                    "readinessProbe": {
                        "httpGet": { "path": "/health", "port": config.sidecar_health_port, "scheme": "HTTP" },
                        "initialDelaySeconds": 5,
                        "periodSeconds": 5
                    }
                }
            ]
        }
    })).unwrap()
}

fn create_workshop_service_spec(
    service_name: &str,
    pod_name: &str,
    user_id: &str,
    workshop_name: &str,
    owner_ref: OwnerReference,
    config: &config::Config,
) -> Service {
    let mut labels = BTreeMap::new();
    labels.insert(LABEL_USER_ID.to_string(), user_id.to_string());
    labels.insert(LABEL_WORKSHOP_NAME.to_string(), workshop_name.to_string());
    labels.insert(LABEL_MANAGED_BY.to_string(), HUB_ID.to_string());

    let mut selector = BTreeMap::new();
    selector.insert("app".to_string(), pod_name.to_string());

    serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": service_name,
            "labels": labels,
            "ownerReferences": [owner_ref]
        },
        "spec": {
            "type": "ClusterIP",
            "selector": selector,
            "ports": [
                { "name": "proxy", "port": config.sidecar_proxy_port, "targetPort": config.sidecar_proxy_port },
                { "name": "health", "port": config.sidecar_health_port, "targetPort": config.sidecar_health_port }
            ]
        }
    })).unwrap()
}