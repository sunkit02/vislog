use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

use crate::data::parsing;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, Error)]
pub enum Error {
    Parsing(String),
}

impl From<parsing::Error> for Error {
    fn from(value: parsing::Error) -> Self {
        Self::Parsing(value.to_string())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
