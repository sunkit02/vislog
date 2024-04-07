use axum::{body::Body, http::Response};
use tracing::{debug, instrument};

use crate::CONFIGS;

#[instrument(skip(res))]
pub async fn mw_set_access_control_allow_origin(mut res: Response<Body>) -> Response<Body> {
    let cors_header_key = "Access-Control-Allow-Origin";

    if let Some(cors) = &CONFIGS.cors {
        if cors.origins.len() >= 1 {
            res.headers_mut().insert(
                cors_header_key,
                cors.origins_to_string()
                    .parse()
                    .expect("Should be valid header value"),
            );
        }

        debug!(
            "Cors header: \"{}: {:?}\"",
            cors_header_key,
            res.headers().get("Access-Control-Allow-Origin")
        );
    }

    res
}
