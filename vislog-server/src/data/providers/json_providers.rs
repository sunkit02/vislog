use std::path::PathBuf;

use serde_json::Value;
use thiserror::{self, Error};

pub trait JsonProvider: Send + Sync {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, Error>;
    fn get_program_json(&self, url: &str) -> Result<Value, Error>;
    fn get_all_course_jsons(&self) -> Result<Vec<Value>, Error>;
    fn get_course_json(&self, url: &str) -> Result<Value, Error>;
}

#[derive(Debug, Error)]
pub enum Error {
    /// Error happened when reading from the file specified by the `path` when
    /// initializing the (FileJsonProvider)[FileJsonProvider]
    Io(#[from] std::io::Error),
    /// Error happened when deserializing into (Value)[serde_json::Value]
    DeserializeFromStr(#[from] serde_json::Error),
    /// Error happened because the format of given JSON didn't fit the expected layout
    Format(&'static str),
    /// File given doesn't exist
    FileNotFound(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone)]
pub struct WebJsonProvider;

impl JsonProvider for WebJsonProvider {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, Error> {
        todo!()
    }

    fn get_program_json(&self, _url: &str) -> Result<Value, Error> {
        todo!()
    }

    fn get_all_course_jsons(&self) -> Result<Vec<Value>, Error> {
        todo!()
    }

    fn get_course_json(&self, _url: &str) -> Result<Value, Error> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct FileJsonProvider {
    data_root: PathBuf,
    all_programs_file: PathBuf,
}

impl FileJsonProvider {
    pub fn init<P: Into<PathBuf>>(data_root: P, all_programs_file: P) -> Result<Self, Error> {
        let data_root: PathBuf = data_root.into();
        let all_programs_file: PathBuf = all_programs_file.into();

        let mut path = data_root.clone();
        path.push(&all_programs_file);

        // Check if the file exists
        if path.try_exists()? == false {
            return Err(Error::FileNotFound(path));
        }

        Ok(Self {
            data_root,
            all_programs_file,
        })
    }
}

impl JsonProvider for FileJsonProvider {
    fn get_all_program_jsons(&self) -> Result<Vec<Value>, Error> {
        let mut path = self.data_root.clone();
        path.push(&self.all_programs_file);

        let json_str = std::fs::read_to_string(path)?;

        let json: Value = serde_json::from_str(&json_str)?;

        // Index into API response to grab the actual JSON array containing the
        // Program Objects which is nested in the format: `obj.programs.program`
        let program_jsons = {
            let Value::Object(json) = json else {
                return Err(Error::Format("expected a JSON object"));
            };

            let (_, programs_json) = json
                .into_iter()
                .filter(|(key, _)| key == "programs")
                .next()
                .ok_or(Error::Format("missing field `programs`"))?;

            let Value::Object(program_json) = programs_json else {
                return Err(Error::Format(
                    "expected field `programs` to be a JSON object",
                ));
            };

            let (_, programs_json) = program_json
                .into_iter()
                .filter(|(key, _)| key == "program")
                .next()
                .ok_or(Error::Format("missing field `program`"))?;

            let Value::Array(program_jsons) = programs_json else {
                return Err(Error::Format("expected field `program` to be a JSON array"));
            };

            program_jsons
        };

        Ok(program_jsons)
    }

    fn get_program_json(&self, url: &str) -> Result<Value, Error> {
        let mut path = self.data_root.clone();
        path.push(url);

        let json_str = std::fs::read_to_string(path)?;

        let program_json: Value = serde_json::from_str(&json_str)?;

        Ok(program_json)
    }

    fn get_all_course_jsons(&self) -> Result<Vec<Value>, Error> {
        let mut path = self.data_root.clone();
        path.push(&self.all_programs_file);

        let json_str = std::fs::read_to_string(path)?;

        let json: Value = serde_json::from_str(&json_str)?;

        // Index into API response to grab the actual JSON array containing the
        // Program Objects which is nested in the format: `obj.programs.program`
        let course_jsons = {
            let Value::Object(json) = json else {
                return Err(Error::Format("expected a JSON object"));
            };

            let (_, courses_json) = json
                .into_iter()
                .filter(|(key, _)| key == "courses")
                .next()
                .ok_or(Error::Format("missing field `courses`"))?;

            let Value::Object(course_json) = courses_json else {
                return Err(Error::Format(
                    "expected field `courses` to be a JSON object",
                ));
            };

            let (_, courses_json) = course_json
                .into_iter()
                .filter(|(key, _)| key == "course")
                .next()
                .ok_or(Error::Format("missing field `course`"))?;

            let Value::Array(course_jsons) = courses_json else {
                return Err(Error::Format("expected field `course` to be a JSON array"));
            };

            course_jsons
        };

        Ok(course_jsons)
    }

    fn get_course_json(&self, _url: &str) -> Result<Value, Error> {
        todo!()
    }
}
