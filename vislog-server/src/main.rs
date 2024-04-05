use data::parsing::json_providers::{FileJsonProvider, JsonProvider};
use vislog_core::Program;

use crate::data::parsing::ProgramsProvider;

mod data;
mod web;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_provider = FileJsonProvider::init("../data".into(), "programs.json".into());
    let program_provider = ProgramsProvider::with(Box::new(json_provider.clone()));

    let (programs, errors) = program_provider.get_all_programs()?;
    dbg!((programs.len(), errors.len()));

    dbg!(errors);

    let cs_major_json = json_provider.get_program_json("cs_major.json")?;
    let cs_major: Program = serde_json::from_str(&(serde_json::to_string(&cs_major_json)?))?;
    dbg!(cs_major.title);

    Ok(())
}
