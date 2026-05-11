//! App configurations API client.

use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::AppConfiguration;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AppsEnvelope {
    data: Vec<AppConfiguration>,
}

#[derive(Debug, Deserialize)]
struct AppEnvelope {
    data: AppConfiguration,
}

impl ApiClient {
    /// GET /cli/v1/infrastructure_projects/:project_id/app_configurations
    pub async fn apps_list(
        &self,
        project_id: i64,
    ) -> Result<Vec<AppConfiguration>, ApiError> {
        let env: AppsEnvelope = self
            .get(&format!(
                "/cli/v1/infrastructure_projects/{project_id}/app_configurations"
            ))
            .await?;
        Ok(env.data)
    }

    /// GET /cli/v1/infrastructure_projects/:project_id/app_configurations/:id
    pub async fn apps_show(
        &self,
        project_id: i64,
        app_id: i64,
    ) -> Result<AppConfiguration, ApiError> {
        let env: AppEnvelope = self
            .get(&format!(
                "/cli/v1/infrastructure_projects/{project_id}/app_configurations/{app_id}"
            ))
            .await?;
        Ok(env.data)
    }
}
