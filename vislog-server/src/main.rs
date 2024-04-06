use std::net::SocketAddr;

use data::parsing::{json_providers::FileJsonProvider, ProgramsProvider};
use tokio::net::TcpListener;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{self, util::SubscriberInitExt};
use tracing_subscriber::{fmt, EnvFilter};

use web::init_server;

mod data;
mod web;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    let json_provider = FileJsonProvider::init("../data", "programs.json");
    let programs_provider = ProgramsProvider::with(Box::new(json_provider));

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    let server = init_server(programs_provider);

    tracing::info!("Listening at {addr}");

    axum::serve(
        listener,
        server.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
