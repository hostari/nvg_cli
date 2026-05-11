//! Typed errors for `/cli/v1` responses.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not authenticated. Run `nvg auth login`.")]
    Unauthorized,
    #[error("payment required: {0}")]
    PaymentRequired(String),
    #[error("not authorized for this resource.")]
    Forbidden,
    #[error("{0} not found.")]
    NotFound(String),
    #[error("validation error: {0}")]
    Unprocessable(String),
    #[error("server error ({0}). Try again or contact support.")]
    Server(u16),
    #[error("could not reach server: {0}")]
    Network(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}
