//! Auth + token management calls against `/cli/v1`.

use crate::api::ApiClient;
use crate::api::error::ApiError;
use crate::api::models::{ApiToken, Viewer};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct ViewerEnvelope {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct TokensEnvelope {
    data: Vec<ApiToken>,
}

#[derive(Debug, Deserialize)]
struct TokenEnvelope {
    token: ApiToken,
}

/// Server response for `POST /cli/v1/device_authorizations`.
#[derive(Debug, Deserialize)]
pub struct DeviceAuthorizationStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Server response for `POST /cli/v1/device_authorizations/poll` on success.
#[derive(Debug, Deserialize)]
pub struct DeviceAuthorizationPollOk {
    pub token: PollTokenPayload,
}

#[derive(Debug, Deserialize)]
pub struct PollTokenPayload {
    pub id: i64,
    pub name: String,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub plaintext: String,
}

/// Distinguishes the three normal poll outcomes the CLI loops over.
#[derive(Debug)]
pub enum PollOutcome {
    Approved(PollTokenPayload),
    Pending,
    Expired,
}

impl ApiClient {
    /// GET /cli/v1/viewer
    pub async fn viewer(&self) -> Result<Viewer, ApiError> {
        let env: ViewerEnvelope = self.get("/cli/v1/viewer").await?;
        Ok(env.viewer)
    }

    /// GET /cli/v1/tokens
    pub async fn tokens_list(&self) -> Result<Vec<ApiToken>, ApiError> {
        let env: TokensEnvelope = self.get("/cli/v1/tokens").await?;
        Ok(env.data)
    }

    /// POST /cli/v1/tokens
    pub async fn tokens_create(
        &self,
        name: &str,
        expires_at: Option<&str>,
    ) -> Result<ApiToken, ApiError> {
        let body = json!({
            "token": { "name": name, "expires_at": expires_at }
        });
        let env: TokenEnvelope = self.post_json("/cli/v1/tokens", &body).await?;
        Ok(env.token)
    }

    /// DELETE /cli/v1/tokens/:id
    pub async fn tokens_revoke(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&format!("/cli/v1/tokens/{id}")).await
    }

    /// POST /cli/v1/device_authorizations
    pub async fn device_authorization_start(&self) -> Result<DeviceAuthorizationStart, ApiError> {
        self.post_json("/cli/v1/device_authorizations", &json!({}))
            .await
    }

    /// POST /cli/v1/device_authorizations/poll
    ///
    /// Distinguishes the three normal outcomes (Approved/Pending/Expired)
    /// without bubbling Pending/Expired up as errors. Other failures still
    /// surface as {ApiError}.
    pub async fn device_authorization_poll(
        &self,
        device_code: &str,
    ) -> Result<PollOutcome, ApiError> {
        use reqwest::header::{ACCEPT, CONTENT_TYPE, USER_AGENT};

        let url = format!("{}/cli/v1/device_authorizations/poll", self.base_url);
        let body = json!({ "device_code": device_code });

        let resp = self
            .http
            .post(url)
            .header(USER_AGENT, concat!("nvg/", env!("CARGO_PKG_VERSION")))
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| ApiError::Unexpected(format!("reading poll body: {e}")))?;

        if status.is_success() {
            let payload: DeviceAuthorizationPollOk = serde_json::from_str(&text)
                .map_err(|e| ApiError::Unexpected(format!("decoding poll body: {e}")))?;
            return Ok(PollOutcome::Approved(payload.token));
        }

        if status == reqwest::StatusCode::BAD_REQUEST {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
            let code = v
                .get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str())
                .unwrap_or("");
            return match code {
                "authorization_pending" => Ok(PollOutcome::Pending),
                "expired_token" => Ok(PollOutcome::Expired),
                other => Err(ApiError::Unexpected(format!("poll error: {other}"))),
            };
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Err(ApiError::NotFound("Device authorization not found.".into()));
        }
        if status.is_server_error() {
            return Err(ApiError::Server(status.as_u16()));
        }
        Err(ApiError::Unexpected(format!(
            "HTTP {} {}",
            status.as_u16(),
            text
        )))
    }
}
