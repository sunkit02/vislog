use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};

use tracing::{debug, info, instrument};
use vislog_core::{parsing::guid::Guid, CourseDetails};

use crate::data::{fetching, providers::courses::CoursesProvider};
use crate::web::error::{Error, Result};

pub fn routes(courses_provider: CoursesProvider) -> Router {
    Router::new()
        .route("/", get(get_all_courses_handler))
        .route("/:guid", get(get_course_handler))
        .route("/refresh", get(refresh_courses_handler))
        .with_state(courses_provider)
}

#[instrument(skip(courses_provider))]
async fn get_all_courses_handler(
    State(courses_provider): State<CoursesProvider>,
) -> Result<Json<Vec<CourseDetails>>> {
    info!("Getting all courses");

    let (courses, errors) = courses_provider.get_all_courses().await?;

    debug!("courses: {}, errors: {}", courses.len(), errors.len());

    Ok(Json(courses))
}

#[instrument(skip(courses_provider))]
async fn get_course_handler(
    Path(guid): Path<Guid>,
    State(courses_provider): State<CoursesProvider>,
) -> Result<Json<CourseDetails>> {
    info!("Getting course with guid: {}", guid);

    let course = courses_provider
        .get_course(&guid)
        .await?
        .ok_or(Error::CourseNotFound(guid))?;

    Ok(Json(course))
}

#[instrument(skip(courses_provider))]
async fn refresh_courses_handler(
    State(courses_provider): State<CoursesProvider>,
) -> Result<Json<Vec<CourseDetails>>> {
    info!("Refreshing all courses");
    let courses = fetching::fetch_all_courses(&courses_provider).await?;

    debug!("Number of courses after refresh: {}", courses.len());

    Ok(Json(courses))
}
