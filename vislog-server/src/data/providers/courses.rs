use std::{collections::HashMap, fmt::Display, sync::Arc};

use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{field::debug, instrument, Level};
use vislog_core::{parsing::guid::Guid, CourseDetails};
use vislog_parser::{parse_courses, ParsingError};

use super::{
    json_providers::{self, JsonProvider},
    ProviderCache,
};

#[derive(Clone)]
pub struct CoursesProvider {
    json_provider: Arc<RwLock<Box<dyn JsonProvider>>>,
    cache: Arc<RwLock<ProviderCache<Guid, CourseDetails, ParsingError>>>,
}

impl CoursesProvider {
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
    pub async fn get_all_courses(&self) -> Result<(Vec<CourseDetails>, Vec<ParsingError>)> {
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

        let courses: Vec<CourseDetails> = cache.items.values().cloned().collect();
        let errors = cache.errors.to_vec();

        Ok((courses, errors))
    }

    #[instrument(level = Level::DEBUG, skip(self))]
    pub async fn get_course(&self, guid: &Guid) -> Result<Option<CourseDetails>> {
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
        mut cache_write_guard: RwLockWriteGuard<
            'a,
            ProviderCache<Guid, CourseDetails, ParsingError>,
        >,
    ) -> Result<()> {
        let course_jsons = json_provider_read_guard.get_all_course_jsons()?;
        let (courses, errors) = parse_courses(course_jsons);

        let programs = courses
            .into_iter()
            .map(|course| (course.guid.clone(), course))
            .collect::<Vec<(Guid, CourseDetails)>>();

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
