use axum::{extract::Path, routing::get, Json, Router};

use error::Result;
use tracing::instrument;
use vislog_core::parsing::guid::Guid;

mod error;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(get_all_courses))
        .route("/:guid", get(get_course))
}

#[instrument]
async fn get_all_courses() -> Result<Json<Vec<()>>> {
    todo!()
}

#[instrument]
async fn get_course(Path(guid): Path<Guid>) -> Result<Json<Vec<()>>> {
    todo!()
}
