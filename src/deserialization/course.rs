use std::mem;

use anyhow::anyhow;
use anyhow::Error as AnyhowError;
use serde::Deserialize;
use thiserror::Error;

use crate::Label;
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
            let mut inner_parser = ParseCoursesState::init();
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
    InitialState(ParsingState),
    CourseDetection(ParsingState),
    InitialBlankRead(ParsingState),
    ReadCourseNoOp(ParsingState),
    OperatorRead(ParsingState),
    ReadCourseWithOp(ParsingState),
    TerminatingBlankRead(ParsingState),
    NestingOperatorRead(ParsingState),
    NestedBlankRead(ParsingState),
    NestedReadCourseNoOp(ParsingState),
    NestedOperatorRead(ParsingState),
    NestedReadCourseWithOp(ParsingState),
    // TODO: Get better name
    NestedIntermediate(ParsingState),
    NestedOperatorSelection(ParsingState),
    NestedCourseDetection(ParsingState),
}

impl ParseCoursesState {
    pub fn init() -> Self {
        ParseCoursesState::InitialState(ParsingState::initial())
    }

    pub fn parse(mut self, entry: ParsedCourseEntry) -> Result<Self, ParseCoursesError> {
        use ParseCoursesError::*;
        use ParseCoursesState::*;

        let res = match self {
            InitialState(mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(InitialBlankRead(state)),
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => {
                    state.course_buffer.get_or_insert(vec![]).push(course);
                    Ok(Self::CourseDetection(state))
                }
            },
            CourseDetection(ref mut state) => match entry {
                ParsedCourseEntry::And => {
                    let _ = state.operator.insert(Operator::And);
                    Ok(Self::OperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Or => {
                    let _ = state.operator.insert(Operator::Or);
                    Ok(Self::OperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Blank => Ok(Self::InitialBlankRead(mem::take(state))),
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(course);
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            InitialBlankRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        // Swap the memory between the new operator group and the free courses in
                        // the `state.course_buffer` and assign the free courses originally in the
                        // `state.course_buffer` to `free_courses`
                        let free_courses = {
                            let mut new_operator_group = vec![course];
                            mem::swap(buf, &mut new_operator_group);
                            new_operator_group
                        };
                        // Convert courses currently in the coure_buffer that are not part of an operator
                        // group into `CourseEntry`(s) and push into `state.entries`
                        let free_course_entries = free_courses
                            .into_iter()
                            .map(|course| CourseEntry::Course(course));

                        state.entries.extend(free_course_entries);

                        // Insert the new

                        Ok(Self::ReadCourseNoOp(mem::take(state)))
                    }
                    None => Ok(Self::ReadCourseNoOp(mem::take(state))),
                },
            },
            ReadCourseNoOp(ref mut state) => match entry {
                ParsedCourseEntry::And => {
                    if let Some(operator) = state.operator {
                        Err(ParsingError(anyhow!(
                            "`operator` should be None at state: {:?}. Got: {:?}",
                            self,
                            operator
                        )))
                    } else {
                        let _ = state.operator.insert(Operator::And);
                        Ok(Self::OperatorRead(mem::take(state)))
                    }
                }
                ParsedCourseEntry::Or => {
                    if let Some(operator) = state.operator {
                        Err(ParsingError(anyhow!(
                            "`operator` should be None at state: {:?}. Got: {:?}",
                            self,
                            operator
                        )))
                    } else {
                        let _ = state.operator.insert(Operator::Or);
                        Ok(Self::OperatorRead(mem::take(state)))
                    }
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(course);
                        Ok(Self::ReadCourseNoOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            OperatorRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(course);
                        Ok(Self::ReadCourseWithOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            ReadCourseWithOp(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(Self::TerminatingBlankRead(mem::take(state))),
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(course);
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            TerminatingBlankRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                    let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    )))?;

                    let courses = CourseEntries(
                        buf.into_iter()
                            .map(|course| CourseEntry::Course(course))
                            .collect(),
                    );

                    let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                        "`operator` should not be None at state: {:?}",
                        state
                    )))?;

                    let operator_entry = match operator {
                        Operator::And => CourseEntry::And(courses),
                        Operator::Or => CourseEntry::Or(courses),
                    };

                    let nesting_entry = match entry {
                        ParsedCourseEntry::And => {
                            CourseEntry::And(CourseEntries(vec![operator_entry]))
                        }
                        ParsedCourseEntry::Or => {
                            CourseEntry::And(CourseEntries(vec![operator_entry]))
                        },
                        invalid_entry => panic!("entry should be either the `ParsedCourseEntry::And` or `ParsedCourseEntry::Or` variants. Got: {:?}", invalid_entry),
                    };

                    state.entries.push(nesting_entry);

                    Ok(NestingOperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(_) => {
                    todo!("Not sure how to handle at the moment")
                }
                ParsedCourseEntry::Course(course) => {
                    // Append parsed Operator group to `state.entries`
                    let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    )))?;
                    let courses = CourseEntries(
                        buf.into_iter()
                            .map(|course| CourseEntry::Course(course))
                            .collect(),
                    );
                    let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                        "`operator` should not be None at state: {:?}",
                        state
                    )))?;
                    let operator_entry = match operator {
                        Operator::And => CourseEntry::And(courses),
                        Operator::Or => CourseEntry::Or(courses),
                    };
                    state.entries.push(operator_entry);

                    // Append new course to new `state.course_buffer`
                    state.course_buffer.insert(Vec::new()).push(course);

                    Ok(Self::CourseDetection(mem::take(state)))
                }
            },
            NestingOperatorRead(_) => todo!(),
            NestedBlankRead(_) => todo!(),
            NestedReadCourseNoOp(_) => todo!(),
            NestedOperatorRead(_) => todo!(),
            NestedReadCourseWithOp(_) => todo!(),
            NestedIntermediate(_) => todo!(),
            NestedOperatorSelection(_) => todo!(),
            NestedCourseDetection(_) => todo!(),
        };

        // dbg!(&res);

        res
    }

    pub fn finish(mut self) -> Result<CourseEntries, ParseCoursesError> {
        use ParseCoursesError::*;

        match self {
            Self::CourseDetection(ref mut state) => {
                let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                    "`course_buf` should not be None at state: {:?}",
                    state
                )))?;

                let courses_iter = buf.into_iter().map(|course| CourseEntry::Course(course));

                let entries = &mut state.entries;
                entries.extend(courses_iter);

                Ok(CourseEntries(mem::take(entries)))
            }
            Self::ReadCourseWithOp(ref mut state) => {
                let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                    "`operator` should not e None at state: {:?}",
                    state
                )))?;

                let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                    "`course_buf` should not be None at state: {:?}",
                    state
                )))?;

                let buf = buf
                    .into_iter()
                    .map(|course| CourseEntry::Course(course))
                    .collect();

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(CourseEntries(buf)),
                    Operator::Or => CourseEntry::Or(CourseEntries(buf)),
                };

                state.entries.push(operator_entry);

                Ok(CourseEntries(mem::take(&mut state.entries)))
            }
            Self::TerminatingBlankRead(ref mut state) => {
                let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                    "`operator` should not e None at state: {:?}",
                    state
                )))?;

                match operator {
                    Operator::And => {
                        let and_entries =
                            CourseEntry::And(CourseEntries(mem::take(&mut state.entries)));
                        Ok(CourseEntries(vec![and_entries]))
                    }
                    Operator::Or => {
                        let or_entries =
                            CourseEntry::Or(CourseEntries(mem::take(&mut state.entries)));
                        Ok(CourseEntries(vec![or_entries]))
                    }
                }
            }
            Self::InitialState(_)
            | Self::InitialBlankRead(_)
            | Self::ReadCourseNoOp(_)
            | Self::OperatorRead(_) => Err(InvalidFinish(self)),
            _ => todo!("Implement nesting in the futre"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Operator {
    And,
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
    Label(Label),
    Course(Course),
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
                    Self::Label(Label {
                        url: entry.url,
                        guid,
                        name: entry.name,
                    })
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

#[derive(Debug, Default)]
pub struct ParsingState {
    pub operator: Option<Operator>,
    pub course_buffer: Option<Vec<Course>>,
    pub entries: Vec<CourseEntry>,
}

impl ParsingState {
    fn initial() -> Self {
        Self {
            operator: None,
            course_buffer: None,
            entries: vec![],
        }
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

    use core::panic;
    use std::fs;

    #[test]
    fn can_parse_program_with_no_operators_and_labels() {
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

        if let Requirement::Courses {
            title,
            entries: courses,
        } = &requirements[0]
        {
            assert_eq!(title.as_str(), "Prerequisites:");
            assert_eq!(courses.0.len(), 2);
        } else {
            panic!("program requirements[0] should be `Requirement::Courses`");
        }

        if let Requirement::Courses {
            title,
            entries: courses,
        } = &requirements[1]
        {
            assert_eq!(title.as_str(), "Major Courses:");
            assert_eq!(courses.0.len(), 20);
        } else {
            panic!("program requirements[1] should be `Requirement::Courses`");
        }
    }

    #[test]
    fn can_parse_program_with_operators_and_without_labels() {
        let program_json =
            fs::read_to_string("./data/computer_information_systems_minor.json").unwrap();
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

        let requirement = if let RequirementModule::SingleBasicRequirement { title, requirement } =
            requirement_module
        {
            assert_eq!(title.as_str(), "Degree Requirements");
            requirement
        } else {
            panic!("program should have `SingleBasicRequirement` variant of `RequirementModule`");
        };

        if let Requirement::Courses { title, entries } = &requirement {
            assert_eq!(title.as_str(), "Minor Requirements:");
            assert_eq!(entries.0.len(), 6);
        } else {
            panic!("program requirement should be `Requirement::Courses`");
        }
    }
}
