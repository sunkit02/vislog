use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Response, StatusCode},
    middleware::map_response,
    response::IntoResponse,
    routing::get,
    Router,
};
use tracing::{info, instrument};

use crate::data::providers::{courses::CoursesProvider, programs::ProgramsProvider};

#[instrument(skip(addr))]
async fn check_health_handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> Response<Body> {
    info!("Check health ping from {:?}", addr);
    StatusCode::OK.into_response()
}

mod api;
mod error;
mod middleware;

pub fn init_server(
    programs_provider: ProgramsProvider,
    courses_provider: CoursesProvider,
) -> Router {
    Router::new()
        .route("/check_health", get(check_health_handler))
        .nest("/api", api::routes(programs_provider, courses_provider))
        .layer(map_response(
            middleware::cors::mw_set_access_control_allow_origin,
        ))
}
