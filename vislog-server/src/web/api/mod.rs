use axum::Router;

use crate::data::parsing::ProgramsProvider;

pub mod error;

mod data;

pub fn routes(pp: ProgramsProvider) -> Router {
    Router::new().nest("/data", data::routes(pp))
}
