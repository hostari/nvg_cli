//! App deployments API client.

use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::{AppDeployment, DeploymentLogs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct DeploymentsPage {
    pub data: Vec<AppDeployment>,
    pub page: u32,
    pub per_page: u32,
    pub total: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommitSummary {
    pub sha: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeploymentCommitsPage {
    pub data: Vec<CommitSummary>,
}

#[derive(Debug, Deserialize)]
struct DeploymentEnvelope {
    data: AppDeployment,
}

#[derive(Debug, Serialize)]
struct DeploymentCreateBody<'a> {
    deployment: DeploymentCreatePayload<'a>,
}

#[derive(Debug, Serialize)]
struct DeploymentCreatePayload<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit_hash: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_build_commands: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_publish_directory: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_version: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docker_image: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dockerfile_path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dockerfile_build_context: Option<&'a str>,
}

impl ApiClient {
    /// GET /cli/v1/infrastructure_projects/:pid/app_configurations/:aid/app_deployments/commits
    pub async fn deployments_commits(
        &self,
        project_id: i64,
        app_id: i64,
        branch: Option<&str>,
        limit: Option<u32>,
    ) -> Result<DeploymentCommitsPage, ApiError> {
        let mut path = format!(
            "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}/app_deployments/commits"
        );
        let mut query: Vec<String> = Vec::new();
        if let Some(b) = branch {
            query.push(format!("branch={b}"));
        }
        if let Some(l) = limit {
            query.push(format!("limit={l}"));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        self.get(&path).await
    }

    /// GET /cli/v1/infrastructure_projects/:pid/app_configurations/:aid/app_deployments
    pub async fn deployments_list(
        &self,
        project_id: i64,
        app_id: i64,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<DeploymentsPage, ApiError> {
        let mut path = format!(
            "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}/app_deployments"
        );
        let mut query: Vec<String> = Vec::new();
        if let Some(p) = page {
            query.push(format!("page={p}"));
        }
        if let Some(pp) = per_page {
            query.push(format!("per_page={pp}"));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }
        self.get(&path).await
    }

    /// GET .../app_deployments/:id_or_version
    ///
    /// `identifier` may be a numeric id or a version_number string like "v3".
    pub async fn deployments_show(
        &self,
        project_id: i64,
        app_id: i64,
        identifier: &str,
    ) -> Result<AppDeployment, ApiError> {
        let env: DeploymentEnvelope = self
            .get(&format!(
                "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}/app_deployments/{identifier}"
            ))
            .await?;
        Ok(env.data)
    }

    /// POST .../app_deployments
    ///
    /// Always submits the "deploy" action server-side. All fields are optional
    /// overrides; the server falls back to AppConfiguration defaults.
    pub async fn deployments_create(
        &self,
        project_id: i64,
        app_id: i64,
        branch: Option<&str>,
        commit: Option<&str>,
        port: Option<u16>,
        build_command: Option<&str>,
        publish_dir: Option<&str>,
        node_version: Option<&str>,
        docker_image: Option<&str>,
        dockerfile: Option<&str>,
        build_context: Option<&str>,
    ) -> Result<AppDeployment, ApiError> {
        let body = DeploymentCreateBody {
            deployment: DeploymentCreatePayload {
                branch_name: branch,
                commit_hash: commit,
                app_port: port,
                node_build_commands: build_command,
                node_publish_directory: publish_dir,
                node_version,
                docker_image,
                dockerfile_path: dockerfile,
                dockerfile_build_context: build_context,
            },
        };
        let env: DeploymentEnvelope = self
            .post_json(
                &format!(
                    "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}/app_deployments"
                ),
                &body,
            )
            .await?;
        Ok(env.data)
    }

    /// GET .../app_deployments/:id_or_version/logs
    pub async fn deployments_logs(
        &self,
        project_id: i64,
        app_id: i64,
        identifier: &str,
    ) -> Result<DeploymentLogs, ApiError> {
        self.get(&format!(
            "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}/app_deployments/{identifier}/logs"
        ))
        .await
    }
}
