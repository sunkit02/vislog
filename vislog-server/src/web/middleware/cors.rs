use axum::{body::Body, http::Response};

use crate::CONFIGS;

pub async fn mw_set_access_control_allow_origin(mut res: Response<Body>) -> Response<Body> {
    if let Some(cors) = &CONFIGS.cors {
        if cors.origins.len() >= 1 {
            res.headers_mut().insert(
                "Access-Control-Allow-Origin",
                cors.origins_to_string()
                    .parse()
                    .expect("Should be valid header value"),
            );
        }
    }

    res
}
