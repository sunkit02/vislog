use std::path::PathBuf;

use serde_json::Value;
use thiserror::{self, Error};

pub trait JsonProvider {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, JsonProviderError>;
    fn get_program_json(&self, url: &str) -> Result<Value, JsonProviderError>;
}

#[derive(Debug, Error)]
pub enum JsonProviderError {
    /// Error happened when reading from the file specified by the `path` when
    /// initializing the (FileJsonProvider)[FileJsonProvider]
    Io(#[from] std::io::Error),
    /// Error happened when deserializing into (Value)[serde_json::Value]
    DeserializeFromStr(#[from] serde_json::Error),
    /// Error happened because the format of given JSON didn't fit the expected layout
    Format(&'static str),
}

impl std::fmt::Display for JsonProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone)]
pub struct WebJsonProvider;

impl JsonProvider for WebJsonProvider {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, JsonProviderError> {
        todo!()
    }

    fn get_program_json(&self, _url: &str) -> Result<Value, JsonProviderError> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct FileJsonProvider {
    data_root: PathBuf,
    all_programs_file: PathBuf,
}

impl FileJsonProvider {
    pub fn init(data_root: PathBuf, all_programs_file: PathBuf) -> Self {
        Self {
            data_root,
            all_programs_file,
        }
    }
}

impl JsonProvider for FileJsonProvider {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, JsonProviderError> {
        let mut path = self.data_root.clone();
        path.push(&self.all_programs_file);

        let json_str = std::fs::read_to_string(path)?;

        let json: Value = serde_json::from_str(&json_str)?;

        // Index into API response to grab the actual JSON array containing the
        // Program Objects which is nested in the format: `obj.programs.program`
        let program_jsons = {
            let Value::Object(json) = json else {
                return Err(JsonProviderError::Format("expected a JSON object"));
            };

            let (_, programs_json) = json
                .into_iter()
                .filter(|(k, _)| k == "programs")
                .next()
                .ok_or(JsonProviderError::Format("missing field `programs`"))?;

            let Value::Object(program_json) = programs_json else {
                return Err(JsonProviderError::Format(
                    "expected field `programs` to be a JSON object",
                ));
            };

            let (_, programs_json) = program_json
                .into_iter()
                .filter(|(k, _)| k == "program")
                .next()
                .ok_or(JsonProviderError::Format("missing field `program`"))?;

            let Value::Array(program_jsons) = programs_json else {
                return Err(JsonProviderError::Format(
                    "expected field `program` to be a JSON array",
                ));
            };

            program_jsons
        };

        Ok(program_jsons)
    }

    fn get_program_json(&self, url: &str) -> Result<Value, JsonProviderError> {
        let mut path = self.data_root.clone();
        path.push(url);

        let json_str = std::fs::read_to_string(path)?;

        let program_json: Value = serde_json::from_str(&json_str)?;

        Ok(program_json)
    }
}
