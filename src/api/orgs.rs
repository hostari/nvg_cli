//! Organizations API client.

use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::Organization;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OrganizationsEnvelope {
    data: Vec<Organization>,
}

impl ApiClient {
    /// GET /cli/v1/organizations
    pub async fn organizations_list(&self) -> Result<Vec<Organization>, ApiError> {
        let env: OrganizationsEnvelope = self.get("/cli/v1/organizations").await?;
        Ok(env.data)
    }
}
