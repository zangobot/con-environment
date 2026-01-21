use crate::config::{Config, Workshop};

/// Validates that we're running in the correct test environment
pub fn validate_talos_environment() -> Result<(), String> {
    // Check for KUBECONFIG
    let kubeconfig = std::env::var("KUBECONFIG")
        .map_err(|_| "KUBECONFIG not set. Please run: export KUBECONFIG=~/.kube/config")?;

    // Check if it points to a Talos config
    if !kubeconfig.contains("talos") && !kubeconfig.contains("test") {
        return Err(format!(
            "KUBECONFIG doesn't appear to be for a test cluster: {}. \
             For safety, tests only run against clusters with 'talos' or 'test' in the path.",
            kubeconfig
        ));
    }

    Ok(())
}

fn test_workshops() -> Vec<Workshop> {
    vec![Workshop {
        name: "workshop".to_string(),
        image: "traefik/whoami".to_string(),
        description: "The host didn't finish setting this up".to_string(),
    }]
}

/// Get base test configuration with reasonable defaults
pub fn get_test_config() -> Config {
    Config {
        workshops: test_workshops(),
        workshop_namespace: "test-workshops".to_string(), // Cross-namespace: workshops go here
        workshop_ttl_seconds: 600,                        // 10 minutes
        workshop_idle_seconds: 120,                       // 2 minutes
        workshop_port: 80,
        workshop_pod_limit: 10,
        workshop_cpu_request: "50m".to_string(),
        workshop_cpu_limit: "200m".to_string(),
        workshop_mem_request: "64Mi".to_string(),
        workshop_mem_limit: "256Mi".to_string(),
        sidecar_health_port: 9000,
        sidecar_proxy_port: 8888,
        garbage_collection_seconds: 300,
    }
}

/// Get configuration optimized for GC tests (shorter timeouts)
pub fn get_gc_test_config() -> Config {
    Config {
        workshops: test_workshops(),
        workshop_namespace: "test-workshops".to_string(), // Cross-namespace
        workshop_ttl_seconds: 0,
        workshop_idle_seconds: 0,
        workshop_port: 80,
        workshop_pod_limit: 5,
        workshop_cpu_request: "50m".to_string(),
        workshop_cpu_limit: "100m".to_string(),
        workshop_mem_request: "32Mi".to_string(),
        workshop_mem_limit: "128Mi".to_string(),
        sidecar_health_port: 9000,
        sidecar_proxy_port: 8888,
        garbage_collection_seconds: 300,
    }
}
