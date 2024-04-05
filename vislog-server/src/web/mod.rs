use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::data::parsing::ProgramsProvider;

async fn check_health_handler() -> Response<Body> {
    StatusCode::OK.into_response()
}

mod api;
mod error;

pub fn init_server(programs_provider: ProgramsProvider) -> Router {
    Router::new()
        .route("/check_health", get(check_health_handler))
        .nest("/api", api::routes(programs_provider))
}
