use std::mem;

use anyhow::anyhow;
use anyhow::Error as AnyhowError;
use serde::Deserialize;
use thiserror::Error;

use crate::{guid::GUID, Course, CourseEntries, CourseEntry};

pub struct CourseParser {
    raw_entries: Option<Vec<RawCourseEntry>>,
}

#[derive(Error, Debug)]
pub enum ParseCoursesError {
    #[error("parse entries terminated at an unexpected state: {0:?}")]
    InvalidFinish(ParseCoursesState),
    #[error("double nesting detected and is not supported")]
    DoubleNesting,
    #[error("invalid entry found: {0:?}")]
    InvalidEntry(ParsedCourseEntry),
    #[error("parser has exhausted all input")]
    ExhaustedParser,
    #[error("an error occurred when parsing: {0}")]
    ParsingError(AnyhowError),
}

impl CourseParser {
    pub fn new(raw_entries: Vec<RawCourseEntry>) -> Self {
        Self {
            raw_entries: Some(raw_entries),
        }
    }

    pub fn parse(&mut self) -> Result<CourseEntries, ParseCoursesError> {
        if let Some(raw_entries) = self.raw_entries.take() {
            let mut inner_parser = ParseCoursesState::CourseDetection(Vec::new());
            // process entries
            for entry in raw_entries {
                let entry = ParsedCourseEntry::try_from(entry)
                    .map_err(|e| ParseCoursesError::ParsingError(e))?;

                inner_parser = inner_parser.parse(entry)?;
            }

            inner_parser.finish()
        } else {
            Err(ParseCoursesError::ExhaustedParser)
        }
    }
}

#[derive(Debug)]
pub enum ParseCoursesState {
    /// Initial state where any combination of inputs are expected
    CourseDetection(Vec<CourseEntry>),
    /// State after reading a `Blank`
    Ready(Vec<CourseEntry>),
    ReadCourseNoOp(Vec<CourseEntry>),
    OperatorRead {
        operator: Operators,
        courses: Vec<CourseEntry>,
    },
    ReadCourseWithOp {
        operator: Operators,
        courses: Vec<CourseEntry>,
    },
    // TODO: Get better name
    Intermediate {
        operator: Operators,
        courses: Vec<CourseEntry>,
    },
    NestedInitial {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedReady {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedReadCourseNoOp {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedOperatorRead {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedReadCourseWithOp {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    // TODO: Get better name
    NestedIntermediate {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedOperatorSelection {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    NestedCourseDetection {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<CourseEntries>)>,
    },
    ReturnResults(CourseEntries),
}

impl ParseCoursesState {
    pub fn new() -> Self {
        ParseCoursesState::CourseDetection(Vec::new())
    }

    pub fn parse(mut self, entry: ParsedCourseEntry) -> Result<Self, ParseCoursesError> {
        use ParseCoursesState::*;

        match self {
            CourseDetection(ref mut courses) => match entry {
                ParsedCourseEntry::And => Err(ParseCoursesError::InvalidEntry(entry)),
                ParsedCourseEntry::Or => Err(ParseCoursesError::InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(Self::Ready(mem::take(courses))),
                ParsedCourseEntry::Label { url, guid, name } => {
                    courses.push(CourseEntry::Label { url, guid, name });
                    Ok(self)
                }
                ParsedCourseEntry::Course(course) => {
                    courses.push(CourseEntry::Course(course));
                    Ok(self)
                }
            },
            Ready(_) => todo!(),
            ReadCourseNoOp(_) => todo!(),
            OperatorRead { operator, courses } => todo!(),
            ReadCourseWithOp { operator, courses } => todo!(),
            Intermediate { operator, courses } => todo!(),
            NestedInitial {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedReady {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedReadCourseNoOp {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedOperatorRead {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedReadCourseWithOp {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedIntermediate {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedOperatorSelection {
                nest_operator,
                nested_courses,
            } => todo!(),
            NestedCourseDetection {
                nest_operator,
                nested_courses,
            } => todo!(),
            ReturnResults(_) => todo!(),
        }
    }

    pub fn finish(self) -> Result<CourseEntries, ParseCoursesError> {
        match self {
            Self::ReturnResults(entries) => Ok(entries),
            Self::CourseDetection(entries) => Ok(CourseEntries(entries)),
            _ => Err(ParseCoursesError::InvalidFinish(self)),
        }
    }
}

#[derive(Debug)]
enum Operators {
    Add,
    Or,
}

#[derive(Debug, Deserialize)]
pub struct RawCourseEntry {
    url: String,
    path: String,
    guid: String,
    name: String,
    number: Option<String>,
    subject_name: Option<String>,
    subject_code: Option<String>,
    credits: String,
    is_narrative: String,
}

#[derive(Debug)]
pub enum ParsedCourseEntry {
    And,
    Or,
    Blank,
    Label {
        url: String,
        guid: GUID,
        name: String,
    },
    Course(crate::Course),
}

impl TryFrom<RawCourseEntry> for ParsedCourseEntry {
    type Error = AnyhowError;

    fn try_from(entry: RawCourseEntry) -> Result<Self, Self::Error> {
        if entry.is_narrative == "True" {
            let parsed_entry = match entry.name.as_str() {
                "And" => Self::And,
                "Or" => Self::Or,
                "" => Self::Blank,
                _ => {
                    let guid = {
                        let guid = entry.guid.as_str();
                        let guid = &guid[1..guid.len() - 1];

                        GUID::try_from(guid)?
                    };
                    Self::Label {
                        url: entry.url,
                        guid,
                        name: entry.name,
                    }
                }
            };

            return Ok(parsed_entry);
        }

        let guid = {
            let guid = entry.guid.as_str();
            let guid = &guid[1..guid.len() - 1];

            GUID::try_from(guid)?
        };

        let number = entry
            .number
            .ok_or(anyhow!("missing course number"))?
            .parse()?;

        let credits = parse_course_credits(entry.credits.as_str())?;

        Ok(Self::Course(Course {
            url: entry.url,
            path: entry.path,
            guid,
            name: entry.name,
            number,
            subject_name: entry.subject_name.ok_or(anyhow!("missing subject name"))?,
            subject_code: entry.subject_code.ok_or(anyhow!("missing subject code"))?,
            credits,
        }))
    }
}

fn parse_course_credits(credits_str: &str) -> Result<(u8, Option<u8>), AnyhowError> {
    let splits = credits_str.split_once('-');

    if let Some((lower, upper)) = splits {
        let lower = lower.parse::<f32>()?;
        let upper = upper.parse::<f32>()?;
        Ok((lower.floor() as u8, Some(upper.floor() as u8)))
    } else {
        Ok((credits_str.parse()?, None))
    }
}

#[cfg(test)]
mod parse_course_credits_test {
    use super::*;

    #[test]
    fn can_parse_single_digit_course_credit() {
        let credits_str = "1";

        assert_eq!(parse_course_credits(credits_str).unwrap(), (1, None));
    }

    #[test]
    fn can_parse_range_of_course_credits() {
        let credits_str = "1.0-3.0";

        assert_eq!(parse_course_credits(credits_str).unwrap(), (1, Some(3)));
    }
}

#[cfg(test)]
mod parse_courses_test {
    use crate::{Program, Requirement, RequirementModule, Requirements};

    use std::fs;

    #[test]
    fn can_parse_courses_with_no_operators() {
        let program_json = fs::read_to_string("./data/cybersecurity_major.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        assert!(matches!(
            parsed_program.requirements,
            Some(Requirements::Single(_))
        ));

        let requirement_module = if let Some(requirements) = parsed_program.requirements {
            if let Requirements::Single(requirement_module) = requirements {
                requirement_module
            } else {
                panic!("program should have `Single` variant of `Requirements`");
            }
        } else {
            panic!("program should have requirements.");
        };

        let requirements = if let RequirementModule::BasicRequirements {
            title,
            requirements,
        } = requirement_module
        {
            assert_eq!(title.as_str(), "Degree Requirements");
            assert_eq!(requirements.len(), 2);
            requirements
        } else {
            panic!("program should have `BasicRequirements` variant of `RequirementModule`");
        };

        if let Requirement::Courses { title, courses } = &requirements[0] {
            assert_eq!(title.as_str(), "Prerequisites:");
            assert_eq!(courses.0.len(), 2);
        } else {
            panic!("program requirements[0] should be `Requirement::Courses`");
        }

        if let Requirement::Courses { title, courses } = &requirements[1] {
            assert_eq!(title.as_str(), "Major Courses:");
            assert_eq!(courses.0.len(), 20);
        } else {
            panic!("program requirements[1] should be `Requirement::Courses`");
        }
    }
}
