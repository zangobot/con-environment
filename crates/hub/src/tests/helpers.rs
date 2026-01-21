use super::config::{get_gc_test_config, get_test_config, validate_talos_environment};
use crate::{config::Config, gc::GarbageCollector, orchestrator::Orchestrator};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};
use kube::{
    api::{DeleteParams, ListParams, PostParams},
    Api, Client,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

/// Main test context that encapsulates all test dependencies
pub struct TestContext {
    pub client: Client,
    pub orchestrator: Orchestrator,
    pub test_namespace: String,
}

impl TestContext {
    /// Create a standard test context
    /// test_name should be the name of the calling test function
    pub async fn new(test_name: &str) -> Self {
        Self::with_config(get_test_config(), test_name).await
    }

    /// Create a test context optimized for GC testing
    pub async fn new_for_gc(test_name: &str) -> Self {
        Self::with_config(get_gc_test_config(), test_name).await
    }

    /// Create a test context with a specific configuration
    async fn with_config(config: Arc<Config>, test_name: &str) -> Self {
        validate_talos_environment()
            .expect("Not running in Talos environment. See README for setup instructions.");

        tracing::info!("🧪 Setting up test context for: {}", test_name);

        let client = Client::try_default()
            .await
            .expect("Failed to create test Kubernetes client. Is the Talos cluster running?");

        // Create a consistent namespace name for this test (no random suffix)
        // This means the same test always uses the same namespace
        let test_namespace = format!("test-{}", test_name.to_lowercase().replace('_', "-"));

        tracing::debug!("Creating/verifying test namespace: {}", test_namespace);

        // Try to create the namespace (idempotent)
        let ns_api: Api<Namespace> = Api::all(client.clone());
        let namespace: Namespace = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Namespace",
            "metadata": {
                "name": test_namespace,
                "labels": {
                    "workshop-hub/test": "true",
                    "workshop-hub/test-name": test_name
                }
            }
        }))
        .unwrap();

        // Create if it doesn't exist, ignore if it already exists
        match ns_api.create(&PostParams::default(), &namespace).await {
            Ok(_) => {
                tracing::info!("✓ Created test namespace: {}", test_namespace);
                // Wait for namespace to be ready
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(kube::Error::Api(err)) if err.code == 409 => {
                // Namespace already exists, that's fine
                tracing::info!("✓ Using existing test namespace: {}", test_namespace);
            }
            Err(e) => {
                tracing::error!("❌ Failed to create test namespace: {}", e);
                panic!("Failed to create test namespace: {}", e);
            }
        }

        // Update config to use this namespace
        let mut config_clone = (*config).clone();
        config_clone.workshop_namespace = test_namespace.clone();
        // Make workshop_name unique to this namespace
        config_clone.workshop_name = format!("{}-test", config_clone.workshop_name);
        let config = config_clone;
        let orchestrator = Orchestrator::with_config(config).await;

        tracing::debug!("Initializing HTTP client for test state");

        let ctx = Self {
            client,
            orchestrator,
            test_namespace,
        };

        tracing::info!("🧹 Clearing test namespace before test execution");
        // Clear the namespace before starting the test
        ctx.clear().await;

        tracing::info!("✅ Test context ready for: {}", test_name);
        ctx
    }

    /// Clear all resources in the test namespace (but keep the namespace)
    /// This is called automatically when creating a test context
    pub async fn clear(&self) {
        tracing::info!("🧹 Clearing test namespace: {}", self.test_namespace);

        // Delete all pods
        let pod_api: Api<Pod> = Api::namespaced(crate::kube_client().await, &self.test_namespace);

        if let Ok(pods) = pod_api.list(&ListParams::default()).await {
            tracing::debug!("Found {} pods to delete", pods.items.len());
            for pod in pods.items {
                if let Some(name) = pod.metadata.name {
                    tracing::trace!("Deleting pod: {}", name);
                    let _ = pod_api.delete(&name, &DeleteParams::default()).await;
                }
            }
        }

        // Delete all services
        let svc_api: Api<Service> =
            Api::namespaced(crate::kube_client().await, &self.test_namespace);

        if let Ok(services) = svc_api.list(&ListParams::default()).await {
            tracing::debug!("Found {} services to delete", services.items.len());
            for service in services.items {
                if let Some(name) = service.metadata.name {
                    tracing::trace!("Deleting service: {}", name);
                    let _ = svc_api.delete(&name, &DeleteParams::default()).await;
                }
            }
        }

        // Wait for deletions to complete
        tracing::debug!("Waiting 2s for resource deletions to propagate");
        tokio::time::sleep(Duration::from_secs(2)).await;

        tracing::info!("✓ Test namespace cleared: {}", self.test_namespace);
    }

    /// Create a test pod with standard labels
    pub async fn create_test_pod(&self, user_id: &str) -> Result<Pod, kube::Error> {
        tracing::info!("Creating test pod for user_id: {}", user_id);

        let pod_api: Api<Pod> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let pod_name = format!("{}-{}", self.config.workshop_name, user_id);

        let mut labels = BTreeMap::new();
        labels.insert("workshop-hub/user-id".to_string(), user_id.to_string());
        labels.insert(
            "workshop-hub/workshop-name".to_string(),
            self.config.workshop_name.clone(),
        );
        labels.insert(
            "app.kubernetes.io/managed-by".to_string(),
            "workshop-hub".to_string(),
        );
        // Add app label for service selector (matches orchestrator pattern)
        labels.insert("app".to_string(), pod_name.clone());

        tracing::debug!("Pod name: {}, labels: {:?}", pod_name, labels);

        let pod: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": pod_name,
                "namespace": self.config.workshop_namespace,
                "labels": labels,
            },
            "spec": {
                "containers": [{
                    "name": "workshop",
                    "image": self.config.workshop_image,
                    "ports": [{
                        "containerPort": self.config.workshop_port
                    }],
                    "resources": {
                        "requests": {
                            "cpu": self.config.workshop_cpu_request,
                            "memory": self.config.workshop_mem_request,
                        },
                        "limits": {
                            "cpu": self.config.workshop_cpu_limit,
                            "memory": self.config.workshop_mem_limit,
                        }
                    }
                }]
            }
        }))
        .unwrap();

        let created_pod = pod_api.create(&PostParams::default(), &pod).await?;

        tracing::info!("✓ Test pod created: {}", pod_name);
        tracing::trace!("Pod details: {:?}", created_pod.metadata);

        Ok(created_pod)
    }

    /// Create a test service for a user pod
    pub async fn create_test_service(&self, user_id: &str) -> Result<Service, kube::Error> {
        tracing::info!("Creating test service for user_id: {}", user_id);

        let svc_api: Api<Service> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let service_name = format!("{}-{}", self.config.workshop_name, user_id);
        let pod_name = format!("{}-{}", self.config.workshop_name, user_id);

        tracing::debug!(
            "Service name: {}, targeting pod: {}",
            service_name,
            pod_name
        );

        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), pod_name);

        let service: Service = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": {
                "name": service_name,
                "namespace": self.config.workshop_namespace,
            },
            "spec": {
                "selector": selector,
                "ports": [{
                    "protocol": "TCP",
                    "port": self.config.workshop_port,
                    "targetPort": self.config.workshop_port,
                }]
            }
        }))
        .unwrap();

        let created_service = svc_api.create(&PostParams::default(), &service).await?;

        tracing::info!("✓ Test service created: {}", service_name);

        Ok(created_service)
    }

    /// Check if a pod exists
    pub async fn pod_exists(&self, pod_name: &str) -> bool {
        tracing::trace!("Checking if pod exists: {}", pod_name);

        let pod_api: Api<Pod> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let exists = pod_api.get(pod_name).await.is_ok();

        tracing::trace!("Pod '{}' exists: {}", pod_name, exists);

        exists
    }

    /// Check if a service exists
    pub async fn service_exists(&self, service_name: &str) -> bool {
        tracing::trace!("Checking if service exists: {}", service_name);

        let svc_api: Api<Service> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let exists = svc_api.get(service_name).await.is_ok();

        tracing::trace!("Service '{}' exists: {}", service_name, exists);

        exists
    }

    /// Count the number of workshop-managed pods in the namespace
    pub async fn count_managed_pods(&self) -> usize {
        tracing::trace!(
            "Counting managed pods in namespace: {}",
            self.test_namespace
        );

        let pod_api: Api<Pod> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let mut list_params = ListParams::default();
        list_params.label_selector = Some(format!(
            "app.kubernetes.io/managed-by=workshop-hub,workshop-hub/workshop-name={}",
            self.config.workshop_name
        ));

        let pods = pod_api
            .list(&list_params)
            .await
            .expect("Failed to list pods");

        let count = pods.items.len();

        tracing::debug!("Found {} managed pods", count);

        count
    }

    /// Wait for a pod to reach running state
    pub async fn wait_for_pod_running(
        &self,
        pod_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("⏳ Waiting for pod to reach Running state: {}", pod_name);

        let pod_api: Api<Pod> =
            Api::namespaced(crate::kube_client().await, &self.config.workshop_namespace);

        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                tracing::error!(
                    "❌ Timeout waiting for pod '{}' to reach Running state",
                    pod_name
                );
                return Err(format!("Timeout waiting for pod {} to be running", pod_name).into());
            }

            match pod_api.get(pod_name).await {
                Ok(pod) => {
                    if let Some(status) = &pod.status {
                        if let Some(phase) = &status.phase {
                            tracing::trace!("Pod '{}' phase: {}", pod_name, phase);

                            if phase == "Running" {
                                tracing::info!("✓ Pod '{}' is Running", pod_name);
                                return Ok(());
                            }

                            if phase == "Failed" || phase == "Unknown" {
                                tracing::error!(
                                    "❌ Pod '{}' entered failed state: {}",
                                    pod_name,
                                    phase
                                );
                                return Err(
                                    format!("Pod {} failed to start: {}", pod_name, phase).into()
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Error getting pod '{}': {}", pod_name, e);
                    return Err(format!("Failed to get pod {}: {}", pod_name, e).into());
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
