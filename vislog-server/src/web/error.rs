use std::sync::Arc;

use axum::response::IntoResponse;
use reqwest::StatusCode;
use thiserror::Error;
use vislog_core::parsing::guid::Guid;

use crate::data::{fetching, providers};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    ProgramsParsing(#[from] providers::programs::Error),
    CoursesParsing(#[from] providers::courses::Error),
    Fetching(#[from] fetching::error::Error),
    ProgramNotFound(Guid),
    CourseNotFound(Guid),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let mut response = StatusCode::INTERNAL_SERVER_ERROR.into_response();

        response.extensions_mut().insert(Arc::new(self));

        response
    }
}

#[macro_export]
macro_rules! impl_generic_error_and_display_for_error_type {
    ($name:ident) => {
        impl std::error::Error for $name {}

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{self:?}")
            }
        }
    };
}
