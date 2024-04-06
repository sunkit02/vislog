use axum::Router;

use crate::data::providers::{courses::CoursesProvider, programs::ProgramsProvider};

pub mod error;

mod courses;
mod programs;

pub fn routes(programs_provider: ProgramsProvider, courses_provider: CoursesProvider) -> Router {
    Router::new()
        .nest("/programs", programs::routes(programs_provider))
        .nest("/courses", courses::routes(courses_provider))
}
