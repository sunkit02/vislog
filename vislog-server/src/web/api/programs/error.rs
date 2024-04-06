use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;
use vislog_core::parsing::guid::Guid;

use crate::data::{fetching, providers};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    ProgramsParsing(#[from] providers::programs::Error),
    Fetching(#[from] fetching::error::Error),
    ProgramNotFound(Guid),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

// TODO: Create better error responses
impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::ProgramsParsing(_) | Error::Fetching(_) => {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            Error::ProgramNotFound(_) => StatusCode::NOT_FOUND.into_response(),
        }
    }
}
