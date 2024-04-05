use std::{fmt::Display, sync::Arc};

use thiserror::Error;
use tokio::sync::RwLock;
use vislog_core::{parsing::guid::GUID, Program};
use vislog_parser::{parse_programs, ProgramParsingError};

pub mod json_providers;

/// Provides program struct parsing
///
/// # Example
///
/// ## Set up using (FileJsonProvider)[json_providers::FileJsonProvider]
/// ```
/// # use vislog_core::{parsing::guid::GUID, Program};
/// # use vislog_parser::{parse_programs, ProgramParsingError};
/// # use self::json_providers::JsonProviderError;
/// let json_provider = FileJsonProvider::init("../data".into(), "programs.json".into());
/// let program_provider = ProgramsProvider::with(Box::new(json_provider.clone()));
/// ```
///
/// ## Get all programs
/// ```
/// # use vislog_core::{parsing::guid::GUID, Program};
/// # use vislog_parser::{parse_programs, ProgramParsingError};
/// # use self::json_providers::JsonProviderError;
/// # let json_provider = FileJsonProvider::init("../data".into(), "programs.json".into());
/// # let program_provider = ProgramsProvider::with(Box::new(json_provider.clone()));
///
/// let (programs, errors) = program_provider.get_all_programs()?;
/// dbg!((programs.len(), errors.len()));
/// ```
///
/// ## Get one program
///
/// ```
/// # use vislog_core::{parsing::guid::GUID, Program};
/// # use vislog_parser::{parse_programs, ProgramParsingError};
/// # use self::json_providers::JsonProviderError;
/// # let json_provider = FileJsonProvider::init("../data".into(), "programs.json".into());
/// # let program_provider = ProgramsProvider::with(Box::new(json_provider.clone()));
/// let cs_major_json = json_provider.get_program_json("cs_major.json")?;
/// let cs_major: Program = serde_json::from_str(&(serde_json::to_string(&cs_major_json)?))?;
/// dbg!(cs_major.title);
/// ```
#[derive(Clone)]
pub struct ProgramsProvider(Arc<RwLock<Box<dyn json_providers::JsonProvider>>>);

impl ProgramsProvider {
    pub fn with(json_provider: Box<dyn json_providers::JsonProvider>) -> Self {
        let provider = Arc::new(RwLock::new(json_provider));
        Self(provider)
    }

    pub async fn get_all_programs(&self) -> Result<(Vec<Program>, Vec<ProgramParsingError>)> {
        let program_jsons = self.0.read().await.get_all_program_jsons()?;
        Ok(parse_programs(program_jsons))
    }

    pub async fn get_program(&self, _guid: GUID) -> Result<Program> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum Error {
    JsonProvider(#[from] json_providers::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
