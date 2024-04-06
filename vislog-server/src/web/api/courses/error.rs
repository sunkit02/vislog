use axum::response::IntoResponse;
use reqwest::StatusCode;
use thiserror::Error;
use vislog_core::parsing::guid::Guid;

use crate::data::{fetching, providers};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    CourseParsing(#[from] providers::courses::Error),
    Fetching(#[from] fetching::error::Error),
    CourseNotFound(Guid),
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
