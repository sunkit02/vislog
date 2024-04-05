use serde_json::{self, Value};
use thiserror::Error;
use vislog_core::Program;

#[derive(Debug, Clone, Error)]
pub enum ProgramParsingError {
    #[error("failed to convert {:?} from value to string because {}", .program_title, .err_msg)]
    Serialization {
        program_title: Option<String>,
        err_msg: String,
    },
    #[error("failed to convert {:?} from value to string because {}", .program_title, .err_msg)]
    Deserialization {
        program_title: Option<String>,
        err_msg: String,
    },
}

pub fn parse_programs<I>(program_jsons: I) -> (Vec<Program>, Vec<ProgramParsingError>)
where
    I: IntoIterator<Item = Value>,
{
    let program_jsons = program_jsons.into_iter();

    let mut errors = vec![];
    let mut programs = Vec::with_capacity(program_jsons.size_hint().0);

    for value in program_jsons {
        let program_title = get_program_title(&value);

        let json_str = {
            match serde_json::to_string_pretty(&value) {
                Ok(json_str) => json_str,
                Err(err) => {
                    errors.push(ProgramParsingError::Serialization {
                        program_title,
                        err_msg: err.to_string(),
                    });
                    // Skip to next program JSON
                    continue;
                }
            }
        };
        match serde_json::from_str::<Program>(&json_str) {
            Ok(program) => programs.push(program),
            Err(err) => errors.push(ProgramParsingError::Deserialization {
                program_title,
                err_msg: err.to_string(),
            }),
        }
    }

    (programs, errors)
}

fn get_program_title(program_json: &Value) -> Option<String> {
    let title_option = if let Value::Object(obj) = program_json {
        if let Some(Value::String(title)) = obj.get("title") {
            Some(title.clone())
        } else {
            None
        }
    } else {
        None
    };

    title_option
}
