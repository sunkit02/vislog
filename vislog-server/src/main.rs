use std::net::SocketAddr;

use data::parsing::{json_providers::FileJsonProvider, ProgramsProvider};
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber;

use web::init_server;

mod data;
mod web;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

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
