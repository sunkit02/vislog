use axum::Router;

use crate::data::parsing::ProgramsProvider;

pub mod error;

mod courses;
mod programs;

pub fn routes(pp: ProgramsProvider) -> Router {
    Router::new()
        .nest("/programs", programs::routes(pp))
        .nest("/courses", courses::routes())
}
