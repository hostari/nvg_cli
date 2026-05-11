//! Serde structs mirroring the JSON shapes documented in cli-plan.md §5.
//!
//! Stub for PR3. Fields will be filled in alongside the endpoints that use them.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewer {
    pub id: i64,
    pub email_address: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: i64,
    pub name: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub current: bool,
    /// Plaintext value, only present in the create response.
    #[serde(default)]
    pub plaintext: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Datacenter {
    pub id: i64,
    pub country: String,
    pub region: String,
    pub city: String,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureProject {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub plan: Option<String>,
    pub datacenter_id: Option<i64>,
    pub organization_id: Option<i64>,
    #[serde(default)]
    pub checkout_status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub checkout_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfiguration {
    pub id: i64,
    pub infrastructure_project_id: i64,
    #[serde(default)]
    pub repository_name: Option<String>,
    #[serde(default)]
    pub repository_full_name: Option<String>,
    #[serde(default)]
    pub deployment_status: Option<String>,
    #[serde(default)]
    pub accessibility_status: Option<String>,
    #[serde(default)]
    pub deployment_flavor: Option<String>,
    #[serde(default)]
    pub fallback_domain_name: Option<String>,
    #[serde(default)]
    pub ip_address: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDeployment {
    pub id: i64,
    pub version_number: String,
    pub status: String,
    pub created_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub commit_sha: Option<String>,
    #[serde(default)]
    pub app_configuration_id: Option<i64>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLogs {
    pub deployment_id: String,
    pub log_string: String,
    pub fetched_at: String,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
}
