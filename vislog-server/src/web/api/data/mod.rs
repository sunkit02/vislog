pub mod error;

use axum::{extract::State, routing::get, Json, Router};
use vislog_core::Program;

use error::Result;

use crate::data::parsing::ProgramsProvider;

pub fn routes(program_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/programs", get(get_all_programs_handler))
        .with_state(program_provider)
}

async fn get_all_programs_handler(
    State(programs_provider): State<ProgramsProvider>,
) -> Result<Json<Vec<Program>>> {
    let (programs, _) = programs_provider.get_all_programs().await?;

    Ok(Json(programs))
}
