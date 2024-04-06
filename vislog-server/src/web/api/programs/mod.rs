pub mod error;

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use tracing::{debug, info, instrument};
use vislog_core::parsing::guid::Guid;
use vislog_core::Program;

use error::Result;

use crate::data::{fetching, parsing::ProgramsProvider};

use self::error::Error;

pub fn routes(program_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/", get(get_all_programs_handler))
        .route("/:guid", get(get_program_handler))
        .route("/refresh", get(refresh_all_programs_handler))
        .with_state(program_provider)
}

#[instrument(skip(programs_provider), err)]
async fn get_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    info!("Getting all programs");

    let (programs, errors) = programs_provider.get_all_programs().await?;

    debug!("programs: {}, errors: {}", programs.len(), errors.len());

    Ok(Json(programs))
}

#[instrument(skip(programs_provider, guid), err)]
async fn get_program_handler(
    State(programs_provider): State<ProgramsProvider>,
    Path(guid): Path<Guid>,
) -> Result<Json<Program>> {
    info!("Getting program with guid: {}", guid);

    let program = programs_provider
        .get_program(&guid)
        .await?
        .ok_or(Error::ProgramNotFound(guid))?;

    Ok(Json(program))
}

// TODO: Update state of ProgramsProvider after fetching the lastest data
#[instrument(err)]
async fn refresh_all_programs_handler() -> Result<Json<Vec<Program>>> {
    info!("Refreshing all programs");
    let programs = fetching::request_all_programs().await?;

    debug!("Number of programs after refresh: {}", programs.len());

    Ok(Json(programs))
}
