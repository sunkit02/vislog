pub mod error;

use axum::{extract::State, routing::get, Json, Router};
use tracing::{debug, info, info_span, instrument};
use vislog_core::Program;

use error::Result;

use crate::data::{fetching, parsing::ProgramsProvider};

pub fn routes(program_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/programs", get(get_all_programs_handler))
        .route("/programs/refresh", get(refresh_all_programs_handler))
        .with_state(program_provider)
}

#[instrument(skip(programs_provider), err)]
async fn get_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    info!("");

    let (programs, errors) = programs_provider.get_all_programs().await?;

    debug!(
        "get_all_programs_handler programs: {}, errors: {}",
        programs.len(),
        errors.len()
    );

    Ok(Json(programs))
}

#[instrument(err)]
async fn refresh_all_programs_handler() -> Result<Json<Vec<Program>>> {
    info!("");
    let programs = fetching::request_all_programs().await?;

    Ok(Json(programs))
}
