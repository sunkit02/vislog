use std::net::SocketAddr;

use data::providers::json_providers::FileJsonProvider;
use lazy_static::lazy_static;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{self, util::SubscriberInitExt};
use tracing_subscriber::{fmt, EnvFilter};

use web::init_server;

use crate::config::ServerConfig;
use crate::data::providers::programs::ProgramsProvider;

mod config;
mod data;
mod web;

lazy_static! {
    static ref DEFAULT_CONFIGS: ServerConfig = ServerConfig::default();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (configs, config_err) = match ServerConfig::new() {
        Ok(configs) => (configs, None),
        Err(err) => (DEFAULT_CONFIGS.clone(), Some(err.to_string())),
    };

    let fmt_layer = fmt::layer().with_target(configs.log.with_target.unwrap_or({
        DEFAULT_CONFIGS
            .log
            .with_target
            .expect("Should be populated")
    }));
    let filter_layer = EnvFilter::new(
        &configs.log.level.unwrap_or(
            DEFAULT_CONFIGS
                .log
                .level
                .as_ref()
                .expect("Should be populated")
                .clone(),
        ),
    );

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    if let Some(err) = config_err {
        error!(
            "Failed to load config file '{}' due to {}",
            config::CONFIG_FILE_PATH,
            err
        );

        info!("Using default configurations");
    } else {
        info!(
            "Successfully loaded config file '{}'",
            config::CONFIG_FILE_PATH,
        );
        info!(
            "Using custom configurations from config file '{}'",
            config::CONFIG_FILE_PATH,
        )
    }

    let json_provider =
        FileJsonProvider::init(&configs.data.storage, &configs.data.all_programs_file);
    let programs_provider = ProgramsProvider::with(Box::new(json_provider));

    let addr = format!("{}:{}", configs.server.host, configs.server.port);
    let listener = TcpListener::bind(&addr).await?;
    let server = init_server(programs_provider);

    info!("Listening at {addr}");

    axum::serve(
        listener,
        server.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
