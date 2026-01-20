use crate::{orchestrator, HubError};
use super::helpers::TestContext;

#[tracing_test::traced_test]
#[tokio::test]
async fn test_orchestrator_creates_pod_and_service() {
    let ctx = TestContext::new("test_orchestrator_creates_pod_and_service").await;

    
    // Test pod creation through orchestrator
    let user_id = "test-user-1";
    let binding = orchestrator::get_or_create_pod(
        &ctx.state.kube_client, 
        user_id, 
        ctx.state.config.clone()
    ).await;
    
    assert!(binding.is_ok(), "Should create pod successfully");
    let binding = binding.unwrap();
    
    // Verify pod was created
    assert!(ctx.pod_exists(&binding.pod_name).await, "Pod should exist");
    
    // Verify service was created
    assert!(ctx.service_exists(&binding.service_name).await, "Service should exist");
    
    // Verify we can retrieve the same binding (idempotency)
    let second_binding = orchestrator::get_or_create_pod(
        &ctx.state.kube_client,
        user_id,
        ctx.state.config.clone()
    ).await;
    
    assert!(second_binding.is_ok(), "Should retrieve existing pod");
    let second_binding = second_binding.unwrap();
    
    assert_eq!(binding.pod_name, second_binding.pod_name, "Should return same pod");
    assert_eq!(binding.service_name, second_binding.service_name, "Should return same service");
}

#[tracing_test::traced_test]
#[tokio::test]
async fn test_pod_limit_enforcement() {
    let ctx = TestContext::new("test_pod_limit_enforcement").await;
    
    // Create a config with very low limit for testing
    let mut limited_config = (*ctx.state.config).clone();
    limited_config.workshop_pod_limit = 2;
    let limited_config = std::sync::Arc::new(limited_config);
    
    // Create pods up to limit
    let user1 = orchestrator::get_or_create_pod(
        &ctx.state.kube_client,
        "limit-user-1",
        limited_config.clone()
    ).await;
    assert!(user1.is_ok(), "First pod should succeed");
    
    let user2 = orchestrator::get_or_create_pod(
        &ctx.state.kube_client,
        "limit-user-2",
        limited_config.clone()
    ).await;
    assert!(user2.is_ok(), "Second pod should succeed");
    
    // This should fail due to limit
    let user3 = orchestrator::get_or_create_pod(
        &ctx.state.kube_client,
        "limit-user-3",
        limited_config.clone()
    ).await;
    
    match user3 {
        Err(HubError::PodLimitReached) => {
            // Expected error
        }
        _ => panic!("Expected PodLimitReached error, got: {:?}", user3),
    }
}


#[tracing_test::traced_test]
#[tokio::test]
async fn test_concurrent_pod_creation() {
    let ctx = TestContext::new("test_concurrent_pod_creation").await;
    
    // Spawn multiple concurrent pod creation requests
    let mut handles = vec![];
    
    for i in 0..5 {
        let client = ctx.state.kube_client.clone();
        let config = ctx.state.config.clone();
        let user_id = format!("concurrent-user-{}", i);
        
        let handle = tokio::spawn(async move {
            orchestrator::get_or_create_pod(&client, &user_id, config).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for result in results {
        assert!(result.is_ok(), "Task should not panic");
        assert!(result.unwrap().is_ok(), "Pod creation should succeed");
    }
    
    // Verify we have exactly 5 pods
    let pod_count = ctx.count_managed_pods().await;
    assert_eq!(pod_count, 5, "Should have exactly 5 pods");
}

// #[tracing_test::traced_test]
// #[tokio::test]
// async fn test_cleanup_preserves_active_pods() {
//     let ctx = TestContext::new_for_gc("test_cleanup_preserves_active_pods").await;
//     let pod_api: Api<Pod> = Api::namespaced(
//         ctx.state.kube_client.clone(),
//         &ctx.state.config.workshop_namespace
//     );
//     let svc_api = kube::Api::<Service>::namespaced(
//          ctx.state.kube_client.clone(),
//         &ctx.state.config.workshop_namespace,
//     );
//     // For this test, we'll assume the GC checks a "last-activity" annotation
//     // In a real scenario, this would come from the sidecar health endpoint
    
//     let result = crate::gc::cleanup_idle_pods(
//         &pod_api,
//         &svc_api,
//         &ctx.state.config.workshop_name,
//         6, // 60 second idle threshold
//     ).await;
    
//     assert!(result.is_ok(), "GC should succeed");
    
//     // In a real test with proper sidecar health endpoints:
//     // - Active pod would report low idle time and remain
//     // - Idle pod would report high idle time and be deleted
// }

