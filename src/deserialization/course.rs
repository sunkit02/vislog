use core::panic;
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
    NestedInitialBlankRead(ParsingState),
    NestedReadCourseNoOp(ParsingState),
    NestedOperatorRead(ParsingState),
    NestedReadCourseWithOp(ParsingState),
    // TODO: Get better name
    NestedTerminatingBlankRead(ParsingState),
}

impl ParseCoursesState {
    pub fn init() -> Self {
        ParseCoursesState::InitialState(ParsingState::initial())
    }

    pub fn parse(mut self, entry: ParsedCourseEntry) -> Result<Self, ParseCoursesError> {
        use ParseCoursesError::*;
        use ParseCoursesState::*;

        let state_name = self.name();

        match self {
            InitialState(mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(InitialBlankRead(state)),
                ParsedCourseEntry::Label(label) => {
                    state
                        .course_buffer
                        .get_or_insert(vec![])
                        .push(CourseEntry::Label(label));
                    Ok(Self::CourseDetection(state))
                }
                ParsedCourseEntry::Course(course) => {
                    state
                        .course_buffer
                        .get_or_insert(vec![])
                        .push(CourseEntry::Course(course));
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
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
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
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        // Swap the memory between the new operator group and the free courses in
                        // the `state.course_buffer` and assign the free courses originally in the
                        // `state.course_buffer` to `free_courses`
                        let free_courses = {
                            let mut new_operator_group = vec![CourseEntry::Label(label)];
                            mem::swap(buf, &mut new_operator_group);
                            new_operator_group
                        };
                        // Convert courses currently in the coure_buffer that are not part of an operator
                        // group into `CourseEntry`(s) and push into `state.entries`
                        state.entries.extend(free_courses);

                        Ok(Self::ReadCourseNoOp(mem::take(state)))
                    }
                    None => Ok(Self::ReadCourseNoOp(mem::take(state))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        // Swap the memory between the new operator group and the free courses in
                        // the `state.course_buffer` and assign the free courses originally in the
                        // `state.course_buffer` to `free_courses`
                        let free_courses = {
                            let mut new_operator_group = vec![CourseEntry::Course(course)];
                            mem::swap(buf, &mut new_operator_group);
                            new_operator_group
                        };
                        // Convert courses currently in the coure_buffer that are not part of an operator
                        // group into `CourseEntry`(s) and push into `state.entries`
                        state.entries.extend(free_courses);

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
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(Self::ReadCourseNoOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
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
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(Self::ReadCourseWithOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(Self::ReadCourseWithOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            ReadCourseWithOp(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                    let current_operator = state.operator.ok_or(ParsingError(anyhow!(
                        "`operator` should not be None at state: {:?}",
                        state_name
                    )))?;

                    let new_operator = match entry {
                        ParsedCourseEntry::And => Operator::And,
                        ParsedCourseEntry::Or => Operator::Or,
                        _ => panic!("This should not happen because the enclosing match condition gurantees that"),
                    };

                    if new_operator == current_operator {
                        Ok(Self::OperatorRead(mem::take(state)))
                    } else {
                        Err(ParsingError(anyhow!(
                            "Expected {:?}, Got {:?}.",
                            current_operator,
                            new_operator
                        )))
                    }
                }
                ParsedCourseEntry::Blank => Ok(Self::TerminatingBlankRead(mem::take(state))),
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
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

                    let courses = CourseEntries(buf);

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
                            CourseEntry::Or(CourseEntries(vec![operator_entry]))
                        },
                        invalid_entry => panic!("entry should be either the `ParsedCourseEntry::And` or `ParsedCourseEntry::Or` variants. Got: {:?}", invalid_entry),
                    };

                    state.entries.push(nesting_entry);

                    Ok(NestingOperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => {
                    // Append parsed Operator group to `state.entries`
                    let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    )))?;
                    let courses = CourseEntries(buf);
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
                    state
                        .course_buffer
                        .insert(Vec::new())
                        .push(CourseEntry::Label(label));

                    Ok(Self::CourseDetection(mem::take(state)))
                }
                ParsedCourseEntry::Course(course) => {
                    // Append parsed Operator group to `state.entries`
                    let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    )))?;
                    let courses = CourseEntries(buf);
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
                    state
                        .course_buffer
                        .insert(Vec::new())
                        .push(CourseEntry::Course(course));

                    Ok(Self::CourseDetection(mem::take(state)))
                }
            },
            NestingOperatorRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(Self::NestedInitialBlankRead(mem::take(state))),
                ParsedCourseEntry::Label(_) => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Course(_) => Err(InvalidEntry(entry)),
            },
            NestedInitialBlankRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => {
                    state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Label(label));

                    Ok(Self::NestedReadCourseNoOp(mem::take(state)))
                }
                ParsedCourseEntry::Course(course) => {
                    state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Course(course));

                    Ok(Self::NestedReadCourseNoOp(mem::take(state)))
                }
            },
            NestedReadCourseNoOp(ref mut state) => match entry {
                ParsedCourseEntry::And => {
                    let _ = state.operator.insert(Operator::And);
                    Ok(Self::NestedOperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Or => {
                    let _ = state.operator.insert(Operator::Or);
                    Ok(Self::NestedOperatorRead(mem::take(state)))
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => {
                    state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Label(label));

                    Ok(self)
                }
                ParsedCourseEntry::Course(course) => {
                    state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Course(course));

                    Ok(self)
                }
            },
            NestedOperatorRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(Self::NestedReadCourseWithOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(Self::NestedReadCourseWithOp(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            NestedReadCourseWithOp(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Ok(Self::NestedTerminatingBlankRead(mem::take(state))),
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(self)
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self
                    ))),
                },
            },
            NestedTerminatingBlankRead(ref mut state) => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                    // Create operator group for courses in the `state.course_buffer` and add it to
                    // the current nesting operator group
                    let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    )))?;

                    let courses = CourseEntries(buf);

                    let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                        "`operator` should not be None at state: {:?}",
                        state
                    )))?;

                    let operator_entry = match operator {
                        Operator::And => CourseEntry::And(courses),
                        Operator::Or => CourseEntry::Or(courses),
                    };

                    let nesting_operator_group = state.entries.last_mut().ok_or(ParsingError(
                        anyhow!("there should be at least one entry in `entries`",),
                    ))?;

                    // Push `operator_entry` into `nesting_operator_group` and get the
                    // `nesting_operator` at the same time
                    let nesting_operator = match nesting_operator_group {
                        CourseEntry::And(group) => {
                            group.0.push(operator_entry);
                            Operator::And
                        }
                        CourseEntry::Or(group) => {
                            group.0.push(operator_entry);
                            Operator::Or
                        }
                        invalid_course_entry => {
                            return Err(ParsingError(anyhow!("Got invalid `CourseEntry` when getting nesting operator group: {:?}", invalid_course_entry)));
                        }
                    };

                    // Determine whether to continue to add to the current nesting operator group
                    // or double nesting has occurred (continue if `nesting_operator` ==
                    // new_operator, double nesting if they differ)
                    let new_operator = match entry {
                            ParsedCourseEntry::And => Operator::And,
                            ParsedCourseEntry::Or => Operator::Or,
                            _ => panic!("`entry` should always be either `ParsedCourseEntry::And` or `ParsedCourseEntry::Or` "),
                        };

                    if nesting_operator == new_operator {
                        Ok(Self::NestingOperatorRead(mem::take(state)))
                    } else {
                        Err(DoubleNesting)
                    }
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                // TODO: Find a way to eliminate the consistent repeating of parsing logic
                // between `Label` and `Course`
                ParsedCourseEntry::Label(label) => match state.course_buffer {
                    Some(ref mut buf) => {
                        // // Swap the memory between the new buffer and the operator group in
                        // // the `state.course_buffer` and assign the operator_group originally in the
                        // // `state.course_buffer` to `operator_group`
                        let operator_group_courses = {
                            let mut new_buffer = vec![CourseEntry::Label(label)];
                            mem::swap(buf, &mut new_buffer);
                            new_buffer
                        };

                        let courses = CourseEntries(operator_group_courses);

                        let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                            "`operator` should not be None at state: {:?}",
                            state
                        )))?;

                        let operator_entry = match operator {
                            Operator::And => CourseEntry::And(courses),
                            Operator::Or => CourseEntry::Or(courses),
                        };

                        let nesting_operator_group =
                            state.entries.last_mut().ok_or(ParsingError(anyhow!(
                                "there should be at least one entry in `entries`",
                            )))?;

                        // Push `operator_entry` into `nesting_operator_group`
                        match nesting_operator_group {
                            CourseEntry::And(group) => {
                                group.0.push(operator_entry);
                            }
                            CourseEntry::Or(group) => {
                                group.0.push(operator_entry);
                            }
                            invalid_course_entry => {
                                return Err(ParsingError(anyhow!("Got invalid `CourseEntry` when getting nesting operator group: {:?}", invalid_course_entry)));
                            }
                        };

                        Ok(Self::CourseDetection(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match state.course_buffer {
                    Some(ref mut buf) => {
                        // // Swap the memory between the new buffer and the operator group in
                        // // the `state.course_buffer` and assign the operator_group originally in the
                        // // `state.course_buffer` to `operator_group`
                        let operator_group_courses = {
                            let mut new_buffer = vec![CourseEntry::Course(course)];
                            mem::swap(buf, &mut new_buffer);
                            new_buffer
                        };

                        let courses = CourseEntries(operator_group_courses);

                        let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                            "`operator` should not be None at state: {:?}",
                            state
                        )))?;

                        let operator_entry = match operator {
                            Operator::And => CourseEntry::And(courses),
                            Operator::Or => CourseEntry::Or(courses),
                        };

                        let nesting_operator_group =
                            state.entries.last_mut().ok_or(ParsingError(anyhow!(
                                "there should be at least one entry in `entries`",
                            )))?;

                        // Push `operator_entry` into `nesting_operator_group`
                        match nesting_operator_group {
                            CourseEntry::And(group) => {
                                group.0.push(operator_entry);
                            }
                            CourseEntry::Or(group) => {
                                group.0.push(operator_entry);
                            }
                            invalid_course_entry => {
                                return Err(ParsingError(anyhow!("Got invalid `CourseEntry` when getting nesting operator group: {:?}", invalid_course_entry)));
                            }
                        };

                        Ok(Self::CourseDetection(mem::take(state)))
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        state
                    ))),
                },
            },
        }
    }

    pub fn finish(mut self) -> Result<CourseEntries, ParseCoursesError> {
        use ParseCoursesError::*;

        match self {
            // Invalid finishing states
            Self::InitialState(_)
            | Self::InitialBlankRead(_)
            | Self::ReadCourseNoOp(_)
            | Self::OperatorRead(_)
            | Self::NestingOperatorRead(_)
            | Self::NestedInitialBlankRead(_)
            | Self::NestedReadCourseNoOp(_)
            | Self::NestedOperatorRead(_) => Err(InvalidFinish(self)),

            // Valid finishing states
            Self::CourseDetection(ref mut state) => {
                let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                    "`course_buf` should not be None at state: {:?}",
                    state
                )))?;

                let entries = &mut state.entries;
                entries.extend(buf);

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
            Self::NestedReadCourseWithOp(ref mut state) => {
                let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                    "`operator` should not e None at state: {:?}",
                    state
                )))?;

                let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                    "`course_buf` should not be None at state: {:?}",
                    state
                )))?;

                let courses = CourseEntries(buf);

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(courses),
                    Operator::Or => CourseEntry::Or(courses),
                };

                let nesting_operator_group = state.entries.last_mut().ok_or(ParsingError(
                    anyhow!("there should be at least one entry in `entries`",),
                ))?;

                match nesting_operator_group {
                    CourseEntry::And(group) => {
                        group.0.push(operator_entry);
                        Operator::And
                    }
                    CourseEntry::Or(group) => {
                        group.0.push(operator_entry);
                        Operator::Or
                    }
                    invalid_course_entry => {
                        return Err(ParsingError(anyhow!(
                            "Got invalid `CourseEntry` when getting nesting operator group: {:?}",
                            invalid_course_entry
                        )));
                    }
                };

                Ok(CourseEntries(mem::take(&mut state.entries)))
            }
            Self::NestedTerminatingBlankRead(ref mut state) => {
                let operator = state.operator.take().ok_or(ParsingError(anyhow!(
                    "`operator` should not e None at state: {:?}",
                    state
                )))?;

                let buf = state.course_buffer.take().ok_or(ParsingError(anyhow!(
                    "`course_buf` should not be None at state: {:?}",
                    state
                )))?;

                let courses = CourseEntries(buf);

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(courses),
                    Operator::Or => CourseEntry::Or(courses),
                };

                let nesting_operator_group = state.entries.last_mut().ok_or(ParsingError(
                    anyhow!("there should be at least one entry in `entries`",),
                ))?;

                match nesting_operator_group {
                    CourseEntry::And(group) => {
                        group.0.push(operator_entry);
                        Operator::And
                    }
                    CourseEntry::Or(group) => {
                        group.0.push(operator_entry);
                        Operator::Or
                    }
                    invalid_course_entry => {
                        return Err(ParsingError(anyhow!(
                            "Got invalid `CourseEntry` when getting nesting operator group: {:?}",
                            invalid_course_entry
                        )));
                    }
                };

                Ok(CourseEntries(mem::take(&mut state.entries)))
            }
        }
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        match self {
            ParseCoursesState::InitialState(_) => "InitialState",
            ParseCoursesState::CourseDetection(_) => "CourseDetection",
            ParseCoursesState::InitialBlankRead(_) => "InitialBlankRead",
            ParseCoursesState::ReadCourseNoOp(_) => "ReadCourseNoOp",
            ParseCoursesState::OperatorRead(_) => "OperatorRead",
            ParseCoursesState::ReadCourseWithOp(_) => "ReadCourseWithOp",
            ParseCoursesState::TerminatingBlankRead(_) => "TerminatingBlankRead",
            ParseCoursesState::NestingOperatorRead(_) => "NestingOperatorRead",
            ParseCoursesState::NestedInitialBlankRead(_) => "NestedInitialBlankRead",
            ParseCoursesState::NestedReadCourseNoOp(_) => "NestedReadCourseNoOp",
            ParseCoursesState::NestedOperatorRead(_) => "NestedOperatorRead",
            ParseCoursesState::NestedReadCourseWithOp(_) => "NestedReadCourseWithOp",
            ParseCoursesState::NestedTerminatingBlankRead(_) => "NestedTerminatingBlankRead",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Operator {
    And,
    Or,
}

#[derive(Debug, Deserialize)]
pub struct RawCourseEntry {
    url: String,
    path: String,
    guid: String,
    name: Option<String>,
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
        if entry.name.is_some() && entry.is_narrative == "True" {
            let parsed_entry = match entry.name.as_ref().unwrap().as_str() {
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
                        name: entry.name.unwrap(),
                        subject_code: entry.subject_code,
                        credits: entry.credits.parse().map_err(|e| {
                            anyhow!("{}", e).context("parsing credits for ParsedCourseEntry::Label")
                        })?,
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
            subject_name: entry.subject_name,
            subject_code: entry.subject_code.ok_or(anyhow!("missing subject code"))?,
            credits,
        }))
    }
}

#[derive(Debug, Default)]
pub struct ParsingState {
    pub operator: Option<Operator>,
    pub course_buffer: Option<Vec<CourseEntry>>,
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

/// Parse `credits` field for `Course` struct from a `&str` containing a valid representation of
/// numbers of credits.
///
/// ### Examples:
/// - "1"
/// - "1.0-3.0"
pub(crate) fn parse_course_credits(credits_str: &str) -> Result<(u8, Option<u8>), AnyhowError> {
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
    use crate::{CourseEntry, Program, Requirement, RequirementModule, Requirements};
    use anyhow::Result;

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
            assert_eq!(title.unwrap().as_str(), "Degree Requirements");
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
            assert_eq!(title.as_ref().unwrap().as_str(), "Prerequisites:");
            assert_eq!(courses.0.len(), 2);
        } else {
            panic!("program requirements[0] should be `Requirement::Courses`");
        }

        if let Requirement::Courses {
            title,
            entries: courses,
        } = &requirements[1]
        {
            assert_eq!(title.as_ref().unwrap().as_str(), "Major Courses:");
            assert_eq!(courses.0.len(), 20);
        } else {
            panic!("program requirements[1] should be `Requirement::Courses`");
        }
    }

    #[test]
    // FIX: Changing the value of "credits" in Course JSON object from string of integers to string of integers with
    // letters or simply letters will cause this test to fail.
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
            assert_eq!(title.unwrap().as_str(), "Degree Requirements");
            requirement
        } else {
            panic!("program should have `SingleBasicRequirement` variant of `RequirementModule`");
        };

        if let Requirement::Courses { title, entries } = &requirement {
            assert_eq!(title.as_ref().unwrap().as_str(), "Minor Requirements:");
            assert_eq!(entries.0.len(), 6);
        } else {
            panic!("program requirement should be `Requirement::Courses`");
        }
    }

    #[test]
    fn can_parse_program_with_nested_operators() {
        let program_json = fs::read_to_string("./data/cs_minor.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        let requirements = if let Some(requirements) = parsed_program.requirements {
            requirements
        } else {
            panic!("`requirements` for program should not be None");
        };

        let req_module = match requirements {
            Requirements::Single(req_module) => req_module,
            requirements => panic!(
                "`requirements` should have `Requirements::Single`. Got: {:?}",
                requirements
            ),
        };

        let requirements = match req_module {
            RequirementModule::BasicRequirements { title, requirements } => {
                assert_eq!(title.unwrap().as_str(), "Degree Requirements");
                assert_eq!(requirements.len(), 3);
                requirements
            }
            req_module => panic!(
                "`requirement_module` should have `RequirementModule::BasicRequirements`. Got: {:?}",
                req_module
            ),
        };

        match &requirements[0] {
            Requirement::Courses { title, entries } => {
                assert_eq!(title.as_ref().unwrap().as_str(), "Minor Requirements:");
                assert_eq!(entries.0.len(), 4);
            }
            invalid_requirement => panic!(
                "`requirement` should have `Requirement::Courses`. Got: {:?}",
                invalid_requirement
            ),
        }

        match &requirements[1] {
            Requirement::Label {
                title,
                req_narrative,
            } => {
                assert_eq!(
                    title.as_ref().unwrap().as_str(),
                    "Select CSC Upper-level Elective: 3 hours"
                );
                assert_eq!(req_narrative, &None);
            }
            invalid_requirement => panic!(
                "`requirement` should have `Requirement::Label`. Got: {:?}",
                invalid_requirement
            ),
        }

        match &requirements[2] {
            Requirement::Courses { title, entries } => {
                assert_eq!(title.as_ref().unwrap().as_str(), "Select one track:");
                assert_eq!(entries.0.len(), 1);
                match &entries.0[0] {
                    CourseEntry::Or(and_course_entries) => {
                        for entry in &and_course_entries.0 {
                            assert!(matches!(entry, CourseEntry::And(_)));
                        }
                    }
                    entry => panic!("Expected `CourseEntry::Or`. Got: {:?}", entry),
                }
            }
            requirement => panic!(
                "`requirement` should have `Requirement::Courses`. Got: {:?}",
                requirement
            ),
        }
    }

    #[test]
    fn can_parse_program_with_chained_homogenous_operators() -> Result<()> {
        let program_json =
            fs::read_to_string("./data/intercultural_strategic_communication.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        let requirements = parsed_program.requirements.unwrap();
        let req_mod = if let Requirements::Single(req_mod) = requirements {
            req_mod
        } else {
            panic!("Expected `Requirements::Single`. Got: {:?}", requirements);
        };

        let req_with_chained_operator = if let RequirementModule::BasicRequirements {
            title,
            requirements,
        } = &req_mod
        {
            assert_eq!(title.as_ref().unwrap().as_str(), "Program Options");
            assert_eq!(requirements.len(), 2);
            &requirements[0]
        } else {
            panic!(
                "Expected `RequirementModule::BasicRequirements`. Got: {:?}",
                req_mod
            );
        };

        if let Requirement::Courses { title, entries } = req_with_chained_operator {
            assert_eq!(
                title.as_ref().unwrap().as_str(),
                "Intercultural Studies Major or Minor with Communication Studies Major:"
            );
            assert_eq!(entries.0.len(), 13);
        } else {
            panic!(
                "Expected `Requirement::Courses`. Got: {:?}",
                req_with_chained_operator
            );
        }

        Ok(())
    }
}
