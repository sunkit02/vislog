pub mod error;

use axum::{extract::State, routing::get, Json, Router};
use vislog_core::Program;

use error::Result;

use crate::data::{fetching, parsing::ProgramsProvider};

pub fn routes(program_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/programs", get(get_all_programs_handler))
        .route("/programs/refresh", get(request_all_programs_handler))
        .with_state(program_provider)
}

async fn get_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    let (programs, _) = programs_provider.get_all_programs().await?;

    Ok(Json(programs))
}

async fn request_all_programs_handler() -> Result<Json<Vec<Program>>> {
    let programs = fetching::request_all_programs().await.unwrap();

    Ok(Json(programs))
}
