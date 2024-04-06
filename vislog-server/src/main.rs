use std::net::SocketAddr;

use data::fetching;
use data::providers::json_providers::FileJsonProvider;
use lazy_static::lazy_static;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{self, util::SubscriberInitExt};
use tracing_subscriber::{fmt, EnvFilter};

use web::init_server;

use crate::configs::{Cors, ServerConfig};
use crate::data::providers::courses::CoursesProvider;
use crate::data::providers::json_providers;
use crate::data::providers::programs::ProgramsProvider;

mod configs;
mod data;
mod web;

lazy_static! {
    pub static ref CONFIGS: ServerConfig = ServerConfig::new().expect(&format!(
        "Failed to load config file '{}'",
        configs::CONFIG_FILE_PATH
    ));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = fmt::layer().with_target(CONFIGS.log.with_target.unwrap_or({
        ServerConfig::default()
            .log
            .with_target
            .expect("Should be populated")
    }));
    let filter_layer = EnvFilter::new(
        CONFIGS.log.level.as_ref().unwrap_or(
            ServerConfig::default()
                .log
                .level
                .as_ref()
                .expect("Should be populated"),
        ),
    );

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // TODO: Figure out why logs in this code block doesn't work
    let programs_provider = {
        let (json_provider, need_refetch) = {
            match FileJsonProvider::init(&CONFIGS.data.storage, &CONFIGS.data.all_programs_file) {
                Ok(provider) => (provider, false),
                Err(json_providers::Error::FileNotFound(path)) => {
                    error!("Given data file '{path:?}' doesn't exist");
                    info!("Creating data file at '{path:?}'");

                    tokio::fs::File::create(&path)
                        .await
                        .expect(&format!("Should be able to create file at {path:?}"));

                    // Try to initialize file provider again. Hard fail if creating data file doesn't
                    // fix the issue
                    let provider = FileJsonProvider::init(
                        &CONFIGS.data.storage,
                        &CONFIGS.data.all_programs_file,
                    )
                    .expect("JsonProvider initialization should succeed after file creation");

                    (provider, true)
                }
                Err(err) => {
                    error!("Failed to initialize JsonProvider: {err}");
                    return Err(err)?;
                }
            }
        };

        let programs_provider = ProgramsProvider::with(Box::new(json_provider));

        if need_refetch {
            info!("Fetching data from {}", CONFIGS.fetching.programs_url);
            fetching::fetch_all_programs(&programs_provider)
                .await
                .expect("Failed to fetch all programs");
        }

        programs_provider
    };

    let courses_provider = {
        let (json_provider, need_refetch) = {
            match FileJsonProvider::init(&CONFIGS.data.storage, &CONFIGS.data.all_courses_file) {
                Ok(provider) => (provider, false),
                Err(json_providers::Error::FileNotFound(path)) => {
                    error!("Given data file '{path:?}' doesn't exist");
                    info!("Creating data file at '{path:?}'");

                    tokio::fs::File::create(&path)
                        .await
                        .expect(&format!("Should be able to create file at {path:?}"));

                    // Try to initialize file provider again. Hard fail if creating data file doesn't
                    // fix the issue
                    let provider = FileJsonProvider::init(
                        &CONFIGS.data.storage,
                        &CONFIGS.data.all_programs_file,
                    )
                    .expect("JsonProvider initialization should succeed after file creation");

                    (provider, true)
                }
                Err(err) => {
                    error!("Failed to initialize JsonProvider: {err}");
                    return Err(err)?;
                }
            }
        };

        let courses_provider = CoursesProvider::with(Box::new(json_provider));

        if need_refetch {
            info!("Fetching data from {}", CONFIGS.fetching.programs_url);
            fetching::fetch_all_courses(&courses_provider)
                .await
                .expect("Failed to fetch all programs");
        }

        courses_provider
    };

    let addr = format!("{}:{}", CONFIGS.server.host, CONFIGS.server.port);
    let listener = TcpListener::bind(&addr).await?;
    let server = init_server(programs_provider, courses_provider);

    info!("Listening at {addr}");

    if let Some(cors) = &CONFIGS.cors {
        if cors.origins.len() >= 1 {
            info!(
                "Allowing requests from origins: \"{}\"",
                cors.origins_to_string()
            );
        }
    }

    axum::serve(
        listener,
        server.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
