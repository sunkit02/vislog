use data::parsing::{json_providers::FileJsonProvider, ProgramsProvider};
use tokio::net::TcpListener;

use web::init_server;

mod data;
mod web;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_provider = FileJsonProvider::init("../data", "programs.json");
    let programs_provider = ProgramsProvider::with(Box::new(json_provider));

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let server = init_server(programs_provider);

    axum::serve(listener, server).await?;

    Ok(())
}
