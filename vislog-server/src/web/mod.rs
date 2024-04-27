use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{HeaderName, Response, StatusCode},
    middleware::map_response,
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
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

#[derive(Debug, Clone, Default)]
struct VislogMakeRequestId {
    counter: Arc<AtomicU64>,
}

impl MakeRequestId for VislogMakeRequestId {
    fn make_request_id<B>(
        &mut self,
        _request: &axum::http::Request<B>,
    ) -> Option<tower_http::request_id::RequestId> {
        let request_id = self
            .counter
            .fetch_add(1, Ordering::SeqCst)
            .to_string()
            .parse()
            .unwrap();

        Some(RequestId::new(request_id))
    }
}

/// Pass in a file path to the directory containing all static assets if you wish to serve static
/// files, otherwise pass in a `None` for `static_dir_path`
pub fn init_server(
    programs_provider: ProgramsProvider,
    courses_provider: CoursesProvider,
    static_dir_path: Option<PathBuf>,
) -> Router {
    let x_request_id = HeaderName::from_static("x-request-d");

    let server = Router::new()
        .route("/check_health", get(check_health_handler))
        .nest("/api", api::routes(programs_provider, courses_provider));

    let server = if let Some(path) = static_dir_path {
        server.nest_service("/", ServeDir::new(path))
    } else {
        server
    };

    server.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(
                x_request_id.clone(),
                VislogMakeRequestId::default(),
            ))
            .layer(PropagateRequestIdLayer::new(x_request_id))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_response(DefaultOnResponse::new().include_headers(true)),
            )
            .layer(map_response(
                middleware::cors::mw_set_access_control_allow_origin,
            )),
    )
}
