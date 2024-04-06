use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};
use vislog_core::parsing::guid::Guid;
use vislog_core::Program;

use crate::web::error::{Error, Result};

use crate::data::{fetching, providers::programs::ProgramsProvider};

pub fn routes(program_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/", get(get_all_programs_handler))
        .route("/:guid", get(get_program_handler))
        .route("/titles", get(get_all_program_titles_handler))
        .route("/refresh", get(refresh_all_programs_handler))
        .with_state(program_provider)
}

#[instrument(skip(programs_provider), err)]
async fn get_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    info!("Getting all programs");

    let (programs, errors) = programs_provider.get_all_programs().await?;

    debug!(
        "Program count: {}, Error count: {}",
        programs.len(),
        errors.len()
    );

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

#[derive(Debug, Deserialize)]
struct ProgramTitlesParam {
    with_guid: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ProgramTitlesResponse {
    WithGuid { guid: Guid, title: String },
    WithoutGuid(String),
}

#[instrument(skip(programs_provider), err)]
async fn get_all_program_titles_handler(
    Query(with_guid): Query<ProgramTitlesParam>,
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<ProgramTitlesResponse>>> {
    info!("Getting all program titles");

    let (programs, _errors) = programs_provider.get_all_programs().await?;
    let with_guid = with_guid.with_guid.unwrap_or(false);

    let responses: Vec<ProgramTitlesResponse> = programs
        .into_iter()
        .map(|p| {
            if with_guid {
                ProgramTitlesResponse::WithGuid {
                    guid: p.guid,
                    title: p.title,
                }
            } else {
                ProgramTitlesResponse::WithoutGuid(p.title)
            }
        })
        .collect();

    debug!("Title count: {}", responses.len());

    Ok(Json(responses))
}

// TODO: Update state of ProgramsProvider after fetching the lastest data
#[instrument(skip(programs_provider), err)]
async fn refresh_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    info!("Refreshing all programs");
    let programs = fetching::fetch_all_programs(&programs_provider).await?;

    debug!("Programs count after refresh: {}", programs.len());

    Ok(Json(programs))
}
