use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::InfrastructureProject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ProjectsEnvelope {
    data: Vec<InfrastructureProject>,
}

#[derive(Debug, Deserialize)]
struct ProjectEnvelope {
    data: InfrastructureProject,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectCreateResult {
    pub project: InfrastructureProject,
    pub checkout_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectCreateEnvelope {
    data: ProjectCreateResult,
}

#[derive(Debug, Serialize)]
struct ProjectCreateBody<'a> {
    project: ProjectCreatePayload<'a>,
}

#[derive(Debug, Serialize)]
struct ProjectCreatePayload<'a> {
    name: &'a str,
    plan_type: &'a str,
    datacenter_id: i64,
    organization_id: i64,
    repository_ids: &'a [String],
}

impl ApiClient {
    /// GET /cli/v1/infrastructure_projects[?organization_id=N]
    pub async fn projects_list(
        &self,
        organization_id: Option<i64>,
    ) -> Result<Vec<InfrastructureProject>, ApiError> {
        let path = match organization_id {
            Some(id) => format!("/cli/v1/infrastructure_projects?organization_id={id}"),
            None => "/cli/v1/infrastructure_projects".to_string(),
        };
        let env: ProjectsEnvelope = self.get(&path).await?;
        Ok(env.data)
    }

    /// GET /cli/v1/infrastructure_projects/:id
    pub async fn projects_show(&self, id: i64) -> Result<InfrastructureProject, ApiError> {
        let env: ProjectEnvelope = self
            .get(&format!("/cli/v1/infrastructure_projects/{id}"))
            .await?;
        Ok(env.data)
    }

    /// DELETE /cli/v1/infrastructure_projects/:id
    pub async fn projects_delete(&self, id: i64) -> Result<(), ApiError> {
        self.delete(&format!("/cli/v1/infrastructure_projects/{id}"))
            .await
    }

    /// POST /cli/v1/infrastructure_projects
    pub async fn projects_create(
        &self,
        name: &str,
        plan_type: &str,
        datacenter_id: i64,
        organization_id: i64,
        repository_ids: &[String],
    ) -> Result<ProjectCreateResult, ApiError> {
        let body = ProjectCreateBody {
            project: ProjectCreatePayload {
                name,
                plan_type,
                datacenter_id,
                organization_id,
                repository_ids,
            },
        };
        let env: ProjectCreateEnvelope = self
            .post_json("/cli/v1/infrastructure_projects", &body)
            .await?;
        Ok(env.data)
    }
}
