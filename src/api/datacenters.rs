//! Datacenters API client.

use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::Datacenter;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DatacentersEnvelope {
    data: Vec<Datacenter>,
}

impl ApiClient {
    /// GET /cli/v1/datacenters
    pub async fn datacenters_list(&self) -> Result<Vec<Datacenter>, ApiError> {
        let env: DatacentersEnvelope = self.get("/cli/v1/datacenters").await?;
        Ok(env.data)
    }
}
