use serde_json::Value;

use reqwest::Result;
use tokio::{fs::File, io::AsyncWriteExt};
use vislog_core::Program;

use crate::data::parsing::{json_providers::FileJsonProvider, ProgramsProvider};

pub async fn request_all_programs() -> Result<Vec<Program>> {
    let url = "https://iq5prod1.smartcatalogiq.com/apis/progAPI?path=/sitecore/content/Catalogs/Union-University/2023/Academic-Catalogue-Undergraduate-Catalogue&format=json";
    let body: Value = reqwest::get(url).await?.json().await?;

    let mut f = File::create("/tmp/programs.json").await.unwrap();
    f.write_all(serde_json::to_string_pretty(&body).unwrap().as_bytes())
        .await
        .unwrap();

    let provider = FileJsonProvider::init("/tmp", "programs.json");
    let provider = ProgramsProvider::with(Box::new(provider));

    let (programs, _errors) = provider.get_all_programs().await.unwrap();

    Ok(programs)
}
