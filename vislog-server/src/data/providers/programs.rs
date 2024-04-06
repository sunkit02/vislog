use std::{collections::HashMap, fmt::Display, sync::Arc};

use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{field::debug, instrument, Level};
use vislog_core::{parsing::guid::Guid, Program};
use vislog_parser::{parse_programs, ParsingError};

use super::{
    json_providers::{self, JsonProvider},
    ProviderCache,
};

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
pub struct ProgramsProvider {
    json_provider: Arc<RwLock<Box<dyn JsonProvider>>>,
    cache: Arc<RwLock<ProviderCache<Guid, Program, ParsingError>>>,
}

impl ProgramsProvider {
    pub fn with(json_provider: Box<dyn JsonProvider>) -> Self {
        let json_provider = Arc::new(RwLock::new(json_provider));
        let cache = ProviderCache {
            items: HashMap::new(),
            errors: Vec::new(),
        };
        let cache = Arc::new(RwLock::new(cache));
        Self {
            json_provider,
            cache,
        }
    }

    #[instrument(skip(self))]
    pub async fn get_all_programs(&self) -> Result<(Vec<Program>, Vec<ParsingError>)> {
        let cache = {
            let read_cache_guard = self.cache.read().await;

            if read_cache_guard.items.is_empty() && read_cache_guard.errors.is_empty() {
                debug("cache empty");
                drop(read_cache_guard);
                let json_provider_read_guard = self.json_provider.read().await;
                let write_cache_guard = self.cache.write().await;
                Self::_refresh_cache(json_provider_read_guard, write_cache_guard).await?;

                // Reacquire read lock
                self.cache.read().await
            } else {
                debug("cache populated");
                read_cache_guard
            }
        };

        let mut programs: Vec<Program> = cache.items.values().cloned().collect();
        programs.sort();
        let errors = cache.errors.to_vec();

        Ok((programs, errors))
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn get_program(&self, guid: &Guid) -> Result<Option<Program>> {
        let cache = {
            let read_cache_guard = self.cache.read().await;

            if read_cache_guard.items.is_empty() && read_cache_guard.errors.is_empty() {
                debug("cache empty");

                drop(read_cache_guard);
                let json_provider_read_guard = self.json_provider.read().await;
                let write_cache_guard = self.cache.write().await;
                Self::_refresh_cache(json_provider_read_guard, write_cache_guard).await?;

                // Reacquire read lock
                self.cache.read().await
            } else {
                debug("cache populated");
                read_cache_guard
            }
        };

        Ok(cache.items.get(guid).map(|p| p.clone()))
    }

    pub async fn refresh_cache(&self) -> Result<()> {
        let json_provider_read_guard = self.json_provider.read().await;
        let cache_write_guard = self.cache.write().await;

        Self::_refresh_cache(json_provider_read_guard, cache_write_guard).await
    }

    /// SAFETY: There must not be a another read guard for `RwLockReadGuard<'a, ProviderCache>` in
    /// the same execution "thread" to avoid deadlocks
    async fn _refresh_cache<'a>(
        json_provider_read_guard: RwLockReadGuard<'a, Box<dyn JsonProvider>>,
        mut cache_write_guard: RwLockWriteGuard<'a, ProviderCache<Guid, Program, ParsingError>>,
    ) -> Result<()> {
        let program_jsons = json_provider_read_guard.get_all_program_jsons()?;
        let (programs, errors) = parse_programs(program_jsons);

        let programs = programs
            .into_iter()
            .map(|p| (p.guid.clone(), p))
            .collect::<Vec<(Guid, Program)>>();

        cache_write_guard.items.clear();
        cache_write_guard.errors.clear();

        cache_write_guard.items.extend(programs);
        cache_write_guard.errors.extend(errors);

        Ok(())
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
