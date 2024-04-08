use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{HeaderName, Request, Response, StatusCode},
    middleware::map_response,
    response::IntoResponse,
    routing::get,
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
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

pub fn init_server(
    programs_provider: ProgramsProvider,
    courses_provider: CoursesProvider,
) -> Router {
    let x_request_id = HeaderName::from_static("x-request-d");

    Router::new()
        .route("/check_health", get(check_health_handler))
        .nest("/api", api::routes(programs_provider, courses_provider))
        .layer(
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
