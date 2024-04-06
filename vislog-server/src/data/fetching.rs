use serde_json::Value;

use tokio::{fs::File, io::AsyncWriteExt};
use vislog_core::Program;

use crate::data::parsing::{json_providers::FileJsonProvider, ProgramsProvider};

use self::error::Result;

pub mod error {
    use std::fmt::Display;

    use thiserror::Error;

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Error)]
    pub enum Error {
        Io(#[from] std::io::Error),
        Parsing(#[from] crate::data::parsing::Error),
        Reqwest(#[from] reqwest::Error),
        SerdeJson(#[from] serde_json::Error),
    }

    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{self:?}")
        }
    }
}

// TODO: Write to a proper storage file
pub async fn request_all_programs() -> Result<Vec<Program>> {
    let url = "https://iq5prod1.smartcatalogiq.com/apis/progAPI?path=/sitecore/content/Catalogs/Union-University/2023/Academic-Catalogue-Undergraduate-Catalogue&format=json";
    let body: Value = reqwest::get(url).await?.json().await?;

    let mut f = File::create("/tmp/programs.json").await.unwrap();
    f.write_all(serde_json::to_string_pretty(&body)?.as_bytes())
        .await?;

    let provider = FileJsonProvider::init("/tmp", "programs.json");
    let provider = ProgramsProvider::with(Box::new(provider));

    let (programs, _errors) = provider.get_all_programs().await?;

    Ok(programs)
}
