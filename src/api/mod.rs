//! HTTP client and typed wrappers around `/cli/v1`.

pub mod apps;
pub mod auth;
pub mod datacenters;
pub mod deployments;
pub mod error;
pub mod models;
pub mod orgs;
pub mod projects;

use error::ApiError;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client as HttpClient, Method, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

const USER_AGENT_VALUE: &str = concat!("nvg/", env!("CARGO_PKG_VERSION"));

pub struct ApiClient {
    pub base_url: String,
    pub token: Option<String>,
    pub http: HttpClient,
}

impl ApiClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        let base_url = base_url.into();
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            base_url,
            token,
            http: HttpClient::new(),
        }
    }

    fn url(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }

    /// GET a JSON resource.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        let resp = self.send::<()>(Method::GET, path, None).await?;
        decode_json(resp).await
    }

    /// POST a JSON body and decode the JSON response.
    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let resp = self.send(Method::POST, path, Some(body)).await?;
        decode_json(resp).await
    }

    /// DELETE a resource. Expects an empty (204) response.
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        self.send::<()>(Method::DELETE, path, None).await?;
        Ok(())
    }

    /// Issue a request, attach auth + standard headers, and convert non-2xx
    /// responses into the appropriate {ApiError}.
    async fn send<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response, ApiError> {
        let mut req = self
            .http
            .request(method, self.url(path))
            .header(USER_AGENT, USER_AGENT_VALUE)
            .header(ACCEPT, "application/json");

        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(b) = body {
            req = req.header(CONTENT_TYPE, "application/json").json(b);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if resp.status().is_success() {
            return Ok(resp);
        }

        Err(map_error(resp).await)
    }
}

async fn decode_json<T: DeserializeOwned>(resp: Response) -> Result<T, ApiError> {
    let text = resp
        .text()
        .await
        .map_err(|e| ApiError::Unexpected(format!("reading response body: {e}")))?;
    if text.is_empty() {
        return serde_json::from_str("null")
            .map_err(|e| ApiError::Unexpected(format!("expected JSON body: {e}")));
    }
    serde_json::from_str(&text).map_err(|e| ApiError::Unexpected(format!("decoding JSON: {e}")))
}

async fn map_error(resp: Response) -> ApiError {
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    let message = body
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let code = body
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    match status {
        StatusCode::UNAUTHORIZED => ApiError::Unauthorized,
        StatusCode::PAYMENT_REQUIRED => ApiError::PaymentRequired(
            body.get("checkout_url")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or(message),
        ),
        StatusCode::FORBIDDEN => ApiError::Forbidden,
        StatusCode::NOT_FOUND => {
            ApiError::NotFound(if message.is_empty() { code } else { message })
        }
        StatusCode::UNPROCESSABLE_ENTITY => ApiError::Unprocessable(message),
        s if s.is_server_error() => ApiError::Server(s.as_u16()),
        s => ApiError::Unexpected(format!("HTTP {} {}", s.as_u16(), message)),
    }
}
