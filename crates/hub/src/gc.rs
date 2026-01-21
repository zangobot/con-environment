use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::watch::Receiver;
use tracing::info;

use pingora::services::background::BackgroundService;

use crate::orchestrator::Orchestrator;

pub struct GarbageCollector;

impl GarbageCollector {
    async fn inner_cleanup_idle_pods(orchestrator: &Orchestrator) -> Result<usize, crate::HubError> {
        // Delegate to the orchestrator's existing GC logic
        orchestrator.gc().await
    }

    #[cfg(test)]
    pub async fn cleanup_idle_pods() -> Result<usize, crate::HubError> {
        let orchestrator = crate::orchestrator().await;
        Self::inner_cleanup_idle_pods(orchestrator).await
    }
}

#[async_trait]
impl BackgroundService for GarbageCollector {
    async fn start(&self, mut shutdown: Receiver<bool>) {
        info!("GC: Starting garbage collector background service");

        let orchestrator = crate::orchestrator().await;
        let gc_interval = Duration::from_secs(orchestrator.config.garbarge_collection_seconds as u64);

        let mut interval = tokio::time::interval(gc_interval);
        // First tick completes immediately; skip it to avoid running GC on startup
        interval.tick().await;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("GC: Running cleanup cycle");
                    match Self::inner_cleanup_idle_pods(orchestrator).await {
                        Ok(count) if count > 0 => {
                            info!("GC: Cleaned up {} idle/expired pods", count);
                        }
                        Ok(_) => {
                            // Nothing to clean, stay quiet
                        }
                        Err(e) => {
                            tracing::error!("GC: Cleanup failed: {}", e);
                        }
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        info!("GC: Received shutdown signal, stopping");
                        break;
                    }
                }
            }
        }

        info!("GC: Background service stopped");
    }
}