use std::time::SystemTime;

use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::Pod;
use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize, Debug, Clone)]
pub struct SidecarHealth {
    pub status: String,
    pub last_activity_timestamp: i64,
    pub idle_seconds: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ManagedPod {
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
    k_pod: Option<Pod>,
    health: Option<SidecarHealth>,
}

impl Default for ManagedPod {
    fn default() -> Self {
        Self { 
            created: SystemTime::UNIX_EPOCH.into(),
            updated: SystemTime::UNIX_EPOCH.into(),
            k_pod: None,
            health: None
        }
    }
}

impl ManagedPod {
    /// Returns true if the pod has been created (timestamp is not 1970).
    pub fn is_alive(&self) -> bool {
        let epoch: DateTime<Utc> = SystemTime::UNIX_EPOCH.into();
        self.created != epoch
    }

    pub fn create(&mut self) {
        self.created = Utc::now();
        self.updated = Utc::now();
    }

    /// Returns the number of seconds since the ManagedPod was created.
    pub fn age(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.created)
            .num_seconds()
    }

    /// Returns health.idle_seconds if available, otherwise returns age.
    pub fn idle(&self) -> i64 {
        match &self.health {
            Some(h) => h.idle_seconds,
            None => self.age(),
        }
    }

    /// Checks if the pod has been idle longer than the threshold.
    pub fn is_expired(&self, threshold_seconds: i64) -> bool {
        self.idle() > threshold_seconds
    }

    /// Updates the Pod and refreshes the updated timestamp.
    pub fn set_pod(&mut self, pod: Pod) {
        self.k_pod = Some(pod);
        self.updated = Utc::now();
    }

    /// Updates the SidecarHealth and refreshes the updated timestamp.
    pub fn set_health(&mut self, health: SidecarHealth) {
        self.health = Some(health);
        self.updated = Utc::now();
    }

    /// Sets created and updated timestamps to 1970 (Unix Epoch).
    pub fn kill(&mut self) {
        let epoch: DateTime<Utc> = SystemTime::UNIX_EPOCH.into();
        self.created = epoch;
        self.updated = epoch;
        self.k_pod = None;
        self.health = None;
    }

    pub fn pod(&self) -> Option<&Pod> {
        self.k_pod.as_ref()
    }

    pub fn health(&self) -> Option<&SidecarHealth> {
        self.health.as_ref()
    }
}
