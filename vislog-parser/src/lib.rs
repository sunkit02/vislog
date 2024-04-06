use serde_json::{self, Value};
use thiserror::Error;
use vislog_core::{Course, CourseDetails, Program};

#[derive(Debug, Clone, Error)]
pub enum ParsingError {
    #[error("failed to convert {:?} from value to string because {}", .title, .err_msg)]
    Serialization {
        title: Option<String>,
        err_msg: String,
    },
    #[error("failed to convert {:?} from value to string because {}", .title, .err_msg)]
    Deserialization {
        title: Option<String>,
        err_msg: String,
    },
}

pub fn parse_programs<I>(program_jsons: I) -> (Vec<Program>, Vec<ParsingError>)
where
    I: IntoIterator<Item = Value>,
{
    let program_jsons = program_jsons.into_iter();

    let mut errors = vec![];
    let mut programs = Vec::with_capacity(program_jsons.size_hint().0);

    for value in program_jsons {
        let program_title = get_program_title(&value);

        let json_str = match serde_json::to_string_pretty(&value) {
            Ok(json_str) => json_str,
            Err(err) => {
                errors.push(ParsingError::Serialization {
                    title: program_title,
                    err_msg: err.to_string(),
                });
                // Skip to next program JSON
                continue;
            }
        };
        match serde_json::from_str::<Program>(&json_str) {
            Ok(program) => programs.push(program),
            Err(err) => errors.push(ParsingError::Deserialization {
                title: program_title,
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

pub fn parse_courses<I>(course_jsons: I) -> (Vec<CourseDetails>, Vec<ParsingError>)
where
    I: IntoIterator<Item = Value>,
{
    let course_jsons = course_jsons.into_iter();

    let mut errors = vec![];
    let mut courses = Vec::with_capacity(course_jsons.size_hint().0);

    for value in course_jsons {
        let course_name = get_course_name(&value);

        let json_str = match serde_json::to_string_pretty(&value) {
            Ok(json_str) => json_str,
            Err(err) => {
                errors.push(ParsingError::Serialization {
                    title: course_name,
                    err_msg: err.to_string(),
                });
                // Skip to next program JSON
                continue;
            }
        };

        match serde_json::from_str::<CourseDetails>(&json_str) {
            Ok(course) => courses.push(course),
            Err(err) => errors.push(ParsingError::Deserialization {
                title: course_name,
                err_msg: err.to_string(),
            }),
        }
    }

    (courses, errors)
}

fn get_course_name(course_json: &Value) -> Option<String> {
    let name_option = if let Value::Object(obj) = course_json {
        if let Some(Value::String(title)) = obj.get("name") {
            Some(title.clone())
        } else {
            None
        }
    } else {
        None
    };

    name_option
}
