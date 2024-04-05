use vislog_core::{parsing::guid::GUID, Program};
use vislog_parser::{parse_programs, ProgramParsingError};

use self::json_providers::JsonProviderError;

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
pub struct ProgramsProvider(Box<dyn json_providers::JsonProvider>);

impl ProgramsProvider {
    pub fn with(json_provider: Box<dyn json_providers::JsonProvider>) -> Self {
        Self(json_provider)
    }

    pub fn get_all_programs(
        &self,
    ) -> Result<(Vec<Program>, Vec<ProgramParsingError>), JsonProviderError> {
        let program_jsons = self.0.get_all_program_jsons()?;
        Ok(parse_programs(program_jsons))
    }

    pub fn get_program(&self, guid: GUID) -> Result<Program, JsonProviderError> {
        todo!()
    }
}
