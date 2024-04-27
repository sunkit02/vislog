use serde_json::Value;

use tokio::{fs::File, io::AsyncWriteExt};
use tracing::debug;
use vislog_core::{Course, CourseDetails, Program};

use crate::{data::providers::programs::ProgramsProvider, CONFIGS};

use self::error::Result;

use super::providers::courses::CoursesProvider;

pub mod error {
    use std::fmt::Display;

    use thiserror::Error;

    use crate::data::providers;

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Error)]
    pub enum Error {
        Io(#[from] std::io::Error),
        ParsingProgram(#[from] providers::programs::Error),
        ParsingCourse(#[from] providers::courses::Error),
        Reqwest(#[from] reqwest::Error),
        SerdeJson(#[from] serde_json::Error),
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{self:?}")
        }
    }
}

// TODO: Remove programs_provider dependency and refresh it's cache elsewhere
// TODO: Do something with the Errors
pub async fn fetch_all_programs(programs_provider: &ProgramsProvider) -> Result<Vec<Program>> {
    // Fetch data from api
    let data_url = &CONFIGS.fetching.programs_url;
    let body: Value = reqwest::get(data_url).await?.json().await?;

    // Write fetched data to storage
    let mut storage_path = CONFIGS.data.storage.clone();
    storage_path.push(&CONFIGS.data.all_programs_file);
    let mut f = File::create(storage_path).await.unwrap();
    f.write_all(serde_json::to_string_pretty(&body)?.as_bytes())
        .await?;
    f.flush().await?;

    // Refresh cache and fetch new results from cache
    programs_provider.refresh_cache().await?;
    let (programs, _errors) = programs_provider.get_all_programs().await?;

    Ok(programs)
}

// TODO: Remove programs_provider dependency and refresh it's cache elsewhere
// TODO: Do something with the Errors
pub async fn fetch_all_courses(courses_provider: &CoursesProvider) -> Result<Vec<CourseDetails>> {
    // Fetch data from api
    let data_url = &CONFIGS.fetching.courses_url;
    let body: Value = reqwest::get(data_url).await?.json().await?;

    // Write fetched data to storage
    let mut storage_path = CONFIGS.data.storage.clone();
    storage_path.push(&CONFIGS.data.all_courses_file);
    let mut f = File::create(storage_path).await.unwrap();
    f.write_all(serde_json::to_string_pretty(&body)?.as_bytes())
        .await?;
    f.flush().await?;

    // Refresh cache and fetch new results from cache
    courses_provider.refresh_cache().await?;
    let (courses, _errors) = courses_provider.get_all_courses().await?;

    Ok(courses)
}
