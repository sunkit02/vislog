use serde_json::{self, Value};
use vislog_core::Program;

#[derive(Debug)]
pub enum ProgramParsingError {
    GenericError {
        program_title: Option<String>,
        error: Box<dyn std::error::Error>,
    },
}

impl std::fmt::Display for ProgramParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ProgramParsingError {}

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
                    errors.push(ProgramParsingError::GenericError {
                        program_title,
                        error: Box::new(err),
                    });
                    // Skip to next program JSON
                    continue;
                }
            }
        };
        match serde_json::from_str::<Program>(&json_str) {
            Ok(program) => programs.push(program),
            Err(err) => errors.push(ProgramParsingError::GenericError {
                program_title,
                error: Box::new(err),
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
