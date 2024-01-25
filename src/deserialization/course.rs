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
    #[error("parse entries terminated at an unexpected state: {:?}", 0)]
    InvalidFinish(ParseCoursesState),
    #[error("double nesting detected and is not supported")]
    DoubleNesting,
    #[error("invalid entry found: {:?}", 0)]
    InvalidEntry(RawCourseEntry),
    #[error("parser has exhausted all input")]
    ExhaustedParser,
    #[error("an error occurred when parsing: {}", 0)]
    ParsingError(#[from] AnyhowError),
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
    CourseDetection(Vec<Course>),
    /// State after reading a `Blank`
    Ready(Vec<Course>),
    ReadCourseNoOp(Vec<Course>),
    OperatorRead {
        operator: Operators,
        courses: Vec<Course>,
    },
    ReadCourseWithOp {
        operator: Operators,
        courses: Vec<Course>,
    },
    // TODO: Get better name
    Intermediate {
        operator: Operators,
        courses: Vec<Course>,
    },
    NestedInitial {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedReady {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedReadCourseNoOp {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedOperatorRead {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedReadCourseWithOp {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    // TODO: Get better name
    NestedIntermediate {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedOperatorSelection {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    NestedCourseDetection {
        nest_operator: Operators,
        nested_courses: Vec<(Operators, Vec<Course>)>,
    },
    ReturnResults(CourseEntries),
}

impl ParseCoursesState {
    pub fn new() -> Self {
        ParseCoursesState::CourseDetection(Vec::new())
    }

    pub fn parse(mut self, item: RawCourseEntry) -> Result<Self, ParseCoursesError> {
        use ParseCoursesState::*;

        match self {
            CourseDetection(courses) => todo!(),
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

pub enum ParsedCourseEntry {
    And,
    Or,
    Blank,
    Label {
        url: String,
        guid: GUID,
        name: String,
    },
    Course {
        url: String,
        path: String,
        guid: GUID,
        name: String,
        number: u16,
        subject_name: String,
        subject_code: u8,
        /// (lower_bound, upper_bound) in other words. lower_bound to upper_bound credits
        credits: (u8, Option<u8>),
    },
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
        let subject_code = entry
            .subject_code
            .ok_or(anyhow!("missing course number"))?
            .parse()?;
        let credits = parse_course_credits(entry.credits.as_str())?;

        Ok(Self::Course {
            url: entry.url,
            path: entry.path,
            guid,
            name: entry.name,
            number,
            subject_name: entry.subject_name.ok_or(anyhow!("missing subject name"))?,
            subject_code,
            credits,
        })
    }
}

fn parse_course_credits(credits_str: &str) -> Result<(u8, Option<u8>), AnyhowError> {
    let mut splits = credits_str.split('-');

    match splits.size_hint().0 {
        // cases like "1"
        1 => Ok((credits_str.parse()?, None)),
        // cases like "1.0-3.0"
        2 => {
            let lower_bound = splits.next().unwrap().parse::<f32>()?;
            let upper_bound = splits.next().unwrap().parse::<f32>()?;
            Ok((lower_bound.floor() as u8, Some(upper_bound.floor() as u8)))
        }
        _ => Err(anyhow!("invalid course credits string: '{}'", credits_str)),
    }
}
