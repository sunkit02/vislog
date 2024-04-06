use core::panic;
use std::mem;

use anyhow::anyhow;
use anyhow::Error as AnyhowError;
use serde::Deserialize;
use thiserror::Error;

use crate::parsing::guid::Guid;
use crate::Label;
use crate::{Course, CourseEntries, CourseEntry};

/// Represents the current state of the course parsing state machine
///
/// NOTE: Important differentiation between `ParseCourseState` and `ParsingState` is that the first one
/// represents the current state of the state machine while the latter stores the data being parsed
/// (`CourseEntries` already parsed, `CourseEntries` currently being worked on, and the `Operator`)
#[derive(Debug)]
pub enum ParseCoursesState {
    InitialState,
    CourseDetection,
    InitialBlankRead,
    ReadCourseNoOp,
    OperatorRead,
    ReadCourseWithOp,
    TerminatingBlankRead,
    NestingOperatorRead,
    NestedInitialBlankRead,
    NestedReadCourseNoOp,
    NestedOperatorRead,
    NestedReadCourseWithOp,
    NestedTerminatingBlankRead,
}

pub struct CoursesParser {
    raw_entries: Vec<RawCourseEntry>,
    state: ParseCoursesState,
    parsing_state: ParsingState,
}

/// Stores the `CourseEntry`s and other information currently/already parsed by the `CourseParser`
#[derive(Debug, Default)]
struct ParsingState {
    /// The operatora relevant to the current Operator Group
    operator: Option<Operator>,
    /// The `CourseEntry`s that are relevant to the current Operator Group
    course_buffer: Option<Vec<CourseEntry>>,
    /// The `CourseEntry`s that have already been parsed (if the last entry is a nesting Operator Group,
    /// it may still be accessed during parsing to append more nested `CourseEntry`s to it)
    entries: Vec<CourseEntry>,
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

impl CoursesParser {
    pub fn new(raw_entries: Vec<RawCourseEntry>) -> Self {
        Self {
            raw_entries,
            state: ParseCoursesState::InitialState,
            parsing_state: ParsingState::initial(),
        }
    }

    /// Consumes the `CoursesParser` struct and parses through all the `RawCourseEntry`s passed in
    /// when initializing the parser.
    ///
    /// NOTE: The `parse` method consumes the `CoursesParser` to avoid having inconsistent statese being
    /// represented and `parse` or `finish` being called in those states
    pub fn parse(mut self) -> Result<CourseEntries, ParseCoursesError> {
        // process entries
        for raw_entry in mem::take(&mut self.raw_entries) {
            let entry =
                ParsedCourseEntry::try_from(raw_entry).map_err(ParseCoursesError::ParsingError)?;

            self.parse_entry(entry)?;
        }

        self.finish()
    }

    pub fn parse_entry(&mut self, entry: ParsedCourseEntry) -> Result<(), ParseCoursesError> {
        use ParseCoursesError::*;
        use ParseCoursesState::*;

        match self.state {
            InitialState => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => return Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => {
                    self.state = InitialBlankRead;
                    Ok(())
                }
                ParsedCourseEntry::Label(label) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(vec![])
                        .push(CourseEntry::Label(label));
                    self.state = CourseDetection;
                    Ok(())
                }
                ParsedCourseEntry::Course(course) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(vec![])
                        .push(CourseEntry::Course(course));
                    self.state = CourseDetection;
                    Ok(())
                }
            },
            CourseDetection => match entry {
                ParsedCourseEntry::And => {
                    let _ = self.parsing_state.operator.insert(Operator::And);
                    self.state = OperatorRead;
                    Ok(())
                }
                ParsedCourseEntry::Or => {
                    let _ = self.parsing_state.operator.insert(Operator::Or);
                    self.state = OperatorRead;
                    Ok(())
                }
                ParsedCourseEntry::Blank => {
                    self.state = InitialBlankRead;
                    Ok(())
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            InitialBlankRead => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
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
                        self.parsing_state.entries.extend(free_courses);

                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                    None => {
                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
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
                        self.parsing_state.entries.extend(free_courses);

                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                    None => {
                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                },
            },
            ReadCourseNoOp => match entry {
                ParsedCourseEntry::And => {
                    if let Some(operator) = self.parsing_state.operator {
                        Err(ParsingError(anyhow!(
                            "`operator` should be None at state: {:?}. Got: {:?}",
                            self.state,
                            operator
                        )))
                    } else {
                        let _ = self.parsing_state.operator.insert(Operator::And);
                        self.state = OperatorRead;
                        Ok(())
                    }
                }
                ParsedCourseEntry::Or => {
                    if let Some(operator) = self.parsing_state.operator {
                        Err(ParsingError(anyhow!(
                            "`operator` should be None at state: {:?}. Got: {:?}",
                            self.state,
                            operator
                        )))
                    } else {
                        let _ = self.parsing_state.operator.insert(Operator::Or);
                        self.state = OperatorRead;
                        Ok(())
                    }
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        self.state = ReadCourseNoOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            OperatorRead => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        self.state = ReadCourseWithOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        self.state = ReadCourseWithOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            ReadCourseWithOp => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                    let current_operator = self.parsing_state.operator.ok_or(ParsingError(
                        anyhow!("`operator` should not be None at state: {:?}", self.state),
                    ))?;

                    let new_operator = match entry {
                        ParsedCourseEntry::And => Operator::And,
                        ParsedCourseEntry::Or => Operator::Or,
                        _ => panic!("This should not happen because the enclosing match condition gurantees that"),
                    };

                    if new_operator == current_operator {
                        self.state = OperatorRead;
                        Ok(())
                    } else {
                        Err(ParsingError(anyhow!(
                            "Expected {:?}, Got {:?}.",
                            current_operator,
                            new_operator
                        )))
                    }
                }
                ParsedCourseEntry::Blank => {
                    self.state = TerminatingBlankRead;
                    Ok(())
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            TerminatingBlankRead => {
                match entry {
                    ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                        let buf = self.parsing_state.course_buffer.take().ok_or(ParsingError(
                            anyhow!("`course_buf` should not be None at state: {:?}", self.state),
                        ))?;

                        let courses = CourseEntries(buf);

                        let operator =
                            self.parsing_state
                                .operator
                                .take()
                                .ok_or(ParsingError(anyhow!(
                                    "`operator` should not be None at state: {:?}",
                                    self.state
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

                        self.parsing_state.entries.push(nesting_entry);

                        self.state = NestingOperatorRead;
                        Ok(())
                    }
                    ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                    ParsedCourseEntry::Label(label) => {
                        // Append parsed Operator group to `state.entries`
                        let buf = self.parsing_state.course_buffer.take().ok_or(ParsingError(
                            anyhow!("`course_buf` should not be None at state: {:?}", self.state),
                        ))?;
                        let courses = CourseEntries(buf);
                        let operator =
                            self.parsing_state
                                .operator
                                .take()
                                .ok_or(ParsingError(anyhow!(
                                    "`operator` should not be None at state: {:?}",
                                    self.state
                                )))?;
                        let operator_entry = match operator {
                            Operator::And => CourseEntry::And(courses),
                            Operator::Or => CourseEntry::Or(courses),
                        };
                        self.parsing_state.entries.push(operator_entry);

                        // Append new course to new `state.course_buffer`
                        self.parsing_state
                            .course_buffer
                            .insert(Vec::new())
                            .push(CourseEntry::Label(label));

                        self.state = CourseDetection;
                        Ok(())
                    }
                    ParsedCourseEntry::Course(course) => {
                        // Append parsed Operator group to `state.entries`
                        let buf = self.parsing_state.course_buffer.take().ok_or(ParsingError(
                            anyhow!("`course_buf` should not be None at state: {:?}", self.state),
                        ))?;
                        let courses = CourseEntries(buf);
                        let operator =
                            self.parsing_state
                                .operator
                                .take()
                                .ok_or(ParsingError(anyhow!(
                                    "`operator` should not be None at state: {:?}",
                                    self.state
                                )))?;
                        let operator_entry = match operator {
                            Operator::And => CourseEntry::And(courses),
                            Operator::Or => CourseEntry::Or(courses),
                        };
                        self.parsing_state.entries.push(operator_entry);

                        // Append new course to new `state.course_buffer`
                        self.parsing_state
                            .course_buffer
                            .insert(Vec::new())
                            .push(CourseEntry::Course(course));

                        self.state = CourseDetection;
                        Ok(())
                    }
                }
            }
            NestingOperatorRead => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => {
                    self.state = NestedInitialBlankRead;
                    Ok(())
                }
                ParsedCourseEntry::Label(_) => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Course(_) => Err(InvalidEntry(entry)),
            },
            NestedInitialBlankRead => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Label(label));

                    self.state = NestedReadCourseNoOp;
                    Ok(())
                }
                ParsedCourseEntry::Course(course) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Course(course));

                    self.state = NestedReadCourseNoOp;
                    Ok(())
                }
            },
            NestedReadCourseNoOp => match entry {
                ParsedCourseEntry::And => {
                    let _ = self.parsing_state.operator.insert(Operator::And);
                    self.state = NestedOperatorRead;
                    Ok(())
                }
                ParsedCourseEntry::Or => {
                    let _ = self.parsing_state.operator.insert(Operator::Or);
                    self.state = NestedOperatorRead;
                    Ok(())
                }
                ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Label(label) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Label(label));

                    Ok(())
                }
                ParsedCourseEntry::Course(course) => {
                    self.parsing_state
                        .course_buffer
                        .get_or_insert(Vec::new())
                        .push(CourseEntry::Course(course));

                    Ok(())
                }
            },
            NestedOperatorRead => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or | ParsedCourseEntry::Blank => {
                    Err(InvalidEntry(entry))
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        self.state = NestedReadCourseWithOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        self.state = NestedReadCourseWithOp;
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            NestedReadCourseWithOp => match entry {
                ParsedCourseEntry::And | ParsedCourseEntry::Or => Err(InvalidEntry(entry)),
                ParsedCourseEntry::Blank => {
                    self.state = NestedTerminatingBlankRead;
                    Ok(())
                }
                ParsedCourseEntry::Label(label) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Label(label));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
                ParsedCourseEntry::Course(course) => match self.parsing_state.course_buffer {
                    Some(ref mut buf) => {
                        buf.push(CourseEntry::Course(course));
                        Ok(())
                    }
                    None => Err(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    ))),
                },
            },
            NestedTerminatingBlankRead => {
                match entry {
                    ParsedCourseEntry::And | ParsedCourseEntry::Or => {
                        // Create operator group for courses in the `state.course_buffer` and add it to
                        // the current nesting operator group
                        let buf = self.parsing_state.course_buffer.take().ok_or(ParsingError(
                            anyhow!("`course_buf` should not be None at state: {:?}", self.state),
                        ))?;

                        let courses = CourseEntries(buf);

                        let operator =
                            self.parsing_state
                                .operator
                                .take()
                                .ok_or(ParsingError(anyhow!(
                                    "`operator` should not be None at state: {:?}",
                                    self.state
                                )))?;

                        let operator_entry = match operator {
                            Operator::And => CourseEntry::And(courses),
                            Operator::Or => CourseEntry::Or(courses),
                        };

                        let nesting_operator_group =
                            self.parsing_state
                                .entries
                                .last_mut()
                                .ok_or(ParsingError(anyhow!(
                                    "there should be at least one entry in `entries`",
                                )))?;

                        // Push `operator_entry` into `nesting_operator_group` and get the
                        // `nesting_operator` at the same time
                        let nesting_operator = match nesting_operator_group {
                            CourseEntry::And(group) => {
                                group.push(operator_entry);
                                Operator::And
                            }
                            CourseEntry::Or(group) => {
                                group.push(operator_entry);
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
                            self.state = NestingOperatorRead;
                            Ok(())
                        } else {
                            Err(DoubleNesting)
                        }
                    }
                    ParsedCourseEntry::Blank => Err(InvalidEntry(entry)),
                    // TODO: Find a way to eliminate the consistent repeating of parsing logic
                    // between `Label` and `Course`
                    ParsedCourseEntry::Label(label) => {
                        match self.parsing_state.course_buffer {
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

                                let operator = self.parsing_state.operator.take().ok_or(
                                    ParsingError(anyhow!(
                                        "`operator` should not be None at state: {:?}",
                                        self.state
                                    )),
                                )?;

                                let operator_entry = match operator {
                                    Operator::And => CourseEntry::And(courses),
                                    Operator::Or => CourseEntry::Or(courses),
                                };

                                let nesting_operator_group =
                                    self.parsing_state.entries.last_mut().ok_or(ParsingError(
                                        anyhow!("there should be at least one entry in `entries`",),
                                    ))?;

                                // Push `operator_entry` into `nesting_operator_group`
                                match nesting_operator_group {
                                    CourseEntry::And(group) => {
                                        group.push(operator_entry);
                                    }
                                    CourseEntry::Or(group) => {
                                        group.push(operator_entry);
                                    }
                                    invalid_course_entry => {
                                        return Err(ParsingError(anyhow!("Got invalid `CourseEntry` when getting nesting operator group: {:?}", invalid_course_entry)));
                                    }
                                };

                                self.state = CourseDetection;
                                Ok(())
                            }
                            None => Err(ParsingError(anyhow!(
                                "`course_buf` should not be None at state: {:?}",
                                self.state
                            ))),
                        }
                    }
                    ParsedCourseEntry::Course(course) => {
                        match self.parsing_state.course_buffer {
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

                                let operator = self.parsing_state.operator.take().ok_or(
                                    ParsingError(anyhow!(
                                        "`operator` should not be None at state: {:?}",
                                        self.state
                                    )),
                                )?;

                                let operator_entry = match operator {
                                    Operator::And => CourseEntry::And(courses),
                                    Operator::Or => CourseEntry::Or(courses),
                                };

                                let nesting_operator_group =
                                    self.parsing_state.entries.last_mut().ok_or(ParsingError(
                                        anyhow!("there should be at least one entry in `entries`",),
                                    ))?;

                                // Push `operator_entry` into `nesting_operator_group`
                                match nesting_operator_group {
                                    CourseEntry::And(group) => {
                                        group.push(operator_entry);
                                    }
                                    CourseEntry::Or(group) => {
                                        group.push(operator_entry);
                                    }
                                    invalid_course_entry => {
                                        return Err(ParsingError(anyhow!("Got invalid `CourseEntry` when getting nesting operator group: {:?}", invalid_course_entry)));
                                    }
                                };

                                self.state = CourseDetection;
                                Ok(())
                            }
                            None => Err(ParsingError(anyhow!(
                                "`course_buf` should not be None at state: {:?}",
                                self.state
                            ))),
                        }
                    }
                }
            }
        }
    }

    /// Call this method when there are no more `RawCourseEntry`s to be processed
    fn finish(mut self) -> Result<CourseEntries, ParseCoursesError> {
        use ParseCoursesError::*;
        use ParseCoursesState::*;

        let entries = match self.state {
            // Invalid finishing states
            InitialState
            | InitialBlankRead
            | ReadCourseNoOp
            | OperatorRead
            | NestingOperatorRead
            | NestedInitialBlankRead
            | NestedReadCourseNoOp
            | NestedOperatorRead => Err(InvalidFinish(self.state)),

            // Valid finishing states
            CourseDetection => {
                let buf = self
                    .parsing_state
                    .course_buffer
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    )))?;

                let entries = &mut self.parsing_state.entries;
                entries.extend(buf);

                Ok(CourseEntries(mem::take(entries)))
            }
            ReadCourseWithOp => {
                let operator = self
                    .parsing_state
                    .operator
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`operator` should not e None at state: {:?}",
                        self.state
                    )))?;

                let buf = self
                    .parsing_state
                    .course_buffer
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    )))?;

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(CourseEntries(buf)),
                    Operator::Or => CourseEntry::Or(CourseEntries(buf)),
                };

                let entries = &mut self.parsing_state.entries;
                entries.push(operator_entry);

                Ok(CourseEntries(mem::take(entries)))
            }
            TerminatingBlankRead => {
                let operator = self
                    .parsing_state
                    .operator
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`operator` should not e None at state: {:?}",
                        self.state
                    )))?;

                match operator {
                    Operator::And => {
                        let and_entries = CourseEntry::And(CourseEntries(mem::take(
                            &mut self.parsing_state.entries,
                        )));
                        Ok(CourseEntries(vec![and_entries]))
                    }
                    Operator::Or => {
                        let or_entries = CourseEntry::Or(CourseEntries(mem::take(
                            &mut self.parsing_state.entries,
                        )));
                        Ok(CourseEntries(vec![or_entries]))
                    }
                }
            }
            NestedReadCourseWithOp => {
                let operator = self
                    .parsing_state
                    .operator
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`operator` should not e None at state: {:?}",
                        self.state
                    )))?;

                let buf = self
                    .parsing_state
                    .course_buffer
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    )))?;

                let courses = CourseEntries(buf);

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(courses),
                    Operator::Or => CourseEntry::Or(courses),
                };

                let nesting_operator_group =
                    self.parsing_state
                        .entries
                        .last_mut()
                        .ok_or(ParsingError(anyhow!(
                            "there should be at least one entry in `entries`",
                        )))?;

                match nesting_operator_group {
                    CourseEntry::And(group) => {
                        group.push(operator_entry);
                        Operator::And
                    }
                    CourseEntry::Or(group) => {
                        group.push(operator_entry);
                        Operator::Or
                    }
                    invalid_course_entry => {
                        return Err(ParsingError(anyhow!(
                            "Got invalid `CourseEntry` when getting nesting operator group: {:?}",
                            invalid_course_entry
                        )));
                    }
                };

                Ok(CourseEntries(mem::take(&mut self.parsing_state.entries)))
            }
            NestedTerminatingBlankRead => {
                let operator = self
                    .parsing_state
                    .operator
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`operator` should not e None at state: {:?}",
                        self.state
                    )))?;

                let buf = self
                    .parsing_state
                    .course_buffer
                    .take()
                    .ok_or(ParsingError(anyhow!(
                        "`course_buf` should not be None at state: {:?}",
                        self.state
                    )))?;

                let courses = CourseEntries(buf);

                let operator_entry = match operator {
                    Operator::And => CourseEntry::And(courses),
                    Operator::Or => CourseEntry::Or(courses),
                };

                let nesting_operator_group =
                    self.parsing_state
                        .entries
                        .last_mut()
                        .ok_or(ParsingError(anyhow!(
                            "there should be at least one entry in `entries`",
                        )))?;

                match nesting_operator_group {
                    CourseEntry::And(group) => {
                        group.push(operator_entry);
                        Operator::And
                    }
                    CourseEntry::Or(group) => {
                        group.push(operator_entry);
                        Operator::Or
                    }
                    invalid_course_entry => {
                        return Err(ParsingError(anyhow!(
                            "Got invalid `CourseEntry` when getting nesting operator group: {:?}",
                            invalid_course_entry
                        )));
                    }
                };

                Ok(CourseEntries(mem::take(&mut self.parsing_state.entries)))
            }
        };

        entries
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

impl ParsedCourseEntry {
    pub fn name(&self) -> &'static str {
        match self {
            ParsedCourseEntry::And => "And",
            ParsedCourseEntry::Or => "Or",
            ParsedCourseEntry::Blank => "Blank",
            ParsedCourseEntry::Label(_) => "Label",
            ParsedCourseEntry::Course(_) => "Course",
        }
    }
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

                        Guid::try_from(guid)?
                    };

                    let credits = parse_course_credits(entry.credits.as_str())?;
                    Self::Label(Label {
                        url: entry.url,
                        guid,
                        name: entry.name.unwrap(),
                        subject_code: entry.subject_code,
                        credits,
                        number: entry.number,
                    })
                }
            };

            return Ok(parsed_entry);
        }

        let guid = {
            let guid = entry.guid.as_str();
            let guid = &guid[1..guid.len() - 1];

            Guid::try_from(guid)?
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
        let program_json = fs::read_to_string("../data/cybersecurity_major.json").unwrap();
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

        if let Requirement::Courses { title, courses } = &requirements[0] {
            assert_eq!(title.as_ref().unwrap().as_str(), "Prerequisites:");
            assert_eq!(courses.0.len(), 2);
        } else {
            panic!("program requirements[0] should be `Requirement::Courses`");
        }

        if let Requirement::Courses { title, courses } = &requirements[1] {
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
            fs::read_to_string("../data/computer_information_systems_minor.json").unwrap();
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

        if let Requirement::Courses { title, courses } = &requirement {
            assert_eq!(title.as_ref().unwrap().as_str(), "Minor Requirements:");
            assert_eq!(courses.len(), 6);
        } else {
            panic!("program requirement should be `Requirement::Courses`");
        }
    }

    #[test]
    fn can_parse_program_with_nested_operators() {
        let program_json = fs::read_to_string("../data/cs_minor.json").unwrap();
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
            Requirement::Courses { title, courses } => {
                assert_eq!(title.as_ref().unwrap().as_str(), "Minor Requirements:");
                assert_eq!(courses.len(), 4);
            }
            invalid_requirement => panic!(
                "`requirement` should have `Requirement::Courses`. Got: {:?}",
                invalid_requirement
            ),
        }

        match &requirements[1] {
            Requirement::SelectFromCourses { title, courses } => {
                assert_eq!(title.as_str(), "Select CSC Upper-level Elective: 3 hours");
                assert_eq!(courses, &None);
            }
            invalid_requirement => panic!(
                "`requirement` should have `Requirement::Label`. Got: {:?}",
                invalid_requirement
            ),
        }

        match &requirements[2] {
            Requirement::SelectFromCourses { title, courses } => {
                assert_eq!(title.as_str(), "Select one track:");
                assert_eq!(courses.as_ref().unwrap().len(), 1);
                match &courses.as_ref().unwrap()[0] {
                    CourseEntry::Or(and_course_entries) => {
                        for entry in and_course_entries.iter() {
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
            fs::read_to_string("../data/intercultural_strategic_communication.json").unwrap();
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

        if let Requirement::Courses { title, courses } = req_with_chained_operator {
            assert_eq!(
                title.as_ref().unwrap().as_str(),
                "Intercultural Studies Major or Minor with Communication Studies Major:"
            );
            assert_eq!(courses.len(), 13);
        } else {
            panic!(
                "Expected `Requirement::Courses`. Got: {:?}",
                req_with_chained_operator
            );
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ParseCoursesError {
    #[error("parse entries terminated at an unexpected state: {0:?}")]
    InvalidFinish(ParseCoursesState),
    #[error("double nesting detected and is not supported")]
    DoubleNesting,
    #[error("invalid entry found: {}", ParsedCourseEntry::name(.0))]
    InvalidEntry(ParsedCourseEntry),
    #[error("parser has exhausted all input")]
    ParserExhausted,
    #[error("an error occurred when parsing: {0}")]
    ParsingError(AnyhowError),
}
