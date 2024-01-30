use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use crate::{
    Course, CourseEntries, CourseEntry, Label, Requirement, RequirementModule, Requirements,
};

use self::{
    course::{parse_course_credits, CourseParser, RawCourseEntry},
    guid::GUID,
};

pub mod course;
pub mod guid;
pub mod select;

impl<'de> Deserialize<'de> for Requirements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RequirementsVisitor)
    }
}

struct RequirementsVisitor;

impl<'de> Visitor<'de> for RequirementsVisitor {
    type Value = Requirements;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON object representing a `RequirementModule` or a JSON array of `RequirementModule`s")
    }

    /// Case for [Requirements::Single] variant
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        /// Intermediate struct used to determine if `requirement_list` is a JSON object or array.
        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum RawRequirement {
            /// Case where the `RequirementModule` only has a single `Course` JSON object in field
            /// `course`
            SingleCourseRequirement(SingleCourseRequirement),
            Single(Requirement),
            Many(Vec<Requirement>),
        }

        #[derive(Debug, Deserialize)]
        struct SingleCourseRequirement {
            title: Option<String>,
            course: Course,
        }

        let mut title: Option<Option<String>> = None;
        let mut req_narrative: Option<Option<String>> = None;
        let mut requirement_list: Option<RawRequirement> = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            match key.as_str() {
                "title" => {
                    if title.is_some() {
                        return Err(de::Error::duplicate_field("title"));
                    }
                    title = Some(map.next_value()?);
                }
                "req_narrative" => {
                    if req_narrative.is_some() {
                        return Err(de::Error::duplicate_field("req_narrative"));
                    }
                    req_narrative = Some(map.next_value()?);
                }
                "requirement_list" => {
                    if requirement_list.is_some() {
                        return Err(de::Error::duplicate_field("requirement_list"));
                    }
                    requirement_list = Some(map.next_value()?);
                }
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>();
                }
            }
        }

        let title = title.ok_or_else(|| de::Error::missing_field("title"))?;

        let requirements =
            requirement_list.ok_or_else(|| de::Error::missing_field("requirements_list"))?;

        let requirement_module = match requirements {
            RawRequirement::Single(requirement) => {
                RequirementModule::SingleBasicRequirement { title, requirement }
            }
            RawRequirement::Many(requirements) => RequirementModule::BasicRequirements {
                title,
                requirements,
            },
            RawRequirement::SingleCourseRequirement(SingleCourseRequirement {
                title: req_title,
                course,
            }) => {
                let requirement = Requirement::Courses {
                    title: req_title,
                    entries: CourseEntries(vec![CourseEntry::Course(course)]),
                };
                RequirementModule::SingleBasicRequirement { title, requirement }
            }
        };

        Ok(Requirements::Single(requirement_module))
    }

    /// Case for [Requirements::Many] variant
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut modules = Vec::new();
        while let Ok(Some(module)) = seq.next_element() {
            modules.push(module);
        }

        Ok(Requirements::Many(modules))
    }
}

impl<'de> Deserialize<'de> for RequirementModule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RequirementModuleVisitor)
    }
}

struct RequirementModuleVisitor;

impl<'de> Visitor<'de> for RequirementModuleVisitor {
    type Value = RequirementModule;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        // TODO: Improve this message
        formatter.write_str("a JSON object representing a program at Union University")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut title: Option<Option<String>> = None;
        let mut requirements: Option<Vec<Requirement>> = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            match key.as_str() {
                "title" => {
                    if title.is_some() {
                        return Err(de::Error::duplicate_field("title"));
                    }
                    title = Some(map.next_value()?);
                }
                "requirement_list" => {
                    if requirements.is_some() {
                        return Err(de::Error::duplicate_field("requirement_list"));
                    }
                    requirements = Some(map.next_value()?);
                }
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>();
                }
            }
        }

        let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
        let requirements = requirements.ok_or_else(|| de::Error::missing_field("requirements"))?;

        Ok(RequirementModule::BasicRequirements {
            title,
            requirements,
        })
    }
}

impl<'de> Deserialize<'de> for Requirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(RequirementVisitor)
    }
}

struct RequirementVisitor;

impl<'de> Visitor<'de> for RequirementVisitor {
    type Value = Requirement;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON object representing a `Requirement` enum")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut title = None;
        let mut req_narrative: Option<Option<String>> = None;
        let mut courses = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            match key.as_str() {
                "title" => {
                    if title.is_some() {
                        return Err(de::Error::duplicate_field("title"));
                    }

                    title = Some(map.next_value()?);
                }
                "req_narrative" => {
                    if req_narrative.is_some() {
                        return Err(de::Error::duplicate_field("req_narrative"));
                    }

                    req_narrative = Some(map.next_value()?);
                }
                "course" => {
                    if courses.is_some() {
                        return Err(de::Error::duplicate_field("course"));
                    }

                    courses = Some(map.next_value()?);
                }
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>();
                }
            }
        }

        // TODO: Implement parsing for `Select` variant
        let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
        let req_narrative =
            req_narrative.ok_or_else(|| de::Error::missing_field("req_narrative"))?;

        let requirement = match courses {
            Some(course_entries) => Requirement::Courses {
                title,
                entries: course_entries,
            },
            None => Requirement::Label {
                title,
                req_narrative,
            },
        };

        Ok(requirement)
    }
}

impl<'de> Deserialize<'de> for CourseEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CourseEntriesVisitor)
    }
}

struct CourseEntriesVisitor;

impl<'de> Visitor<'de> for CourseEntriesVisitor {
    type Value = CourseEntries;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of JSON objects representing a `SelectionEntry`")
    }

    // Normal code path for `Requirement`s with a JSON array of `Course` objects in `course` field
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut raw_entries = Vec::with_capacity(seq.size_hint().unwrap_or(4) as usize);

        while let Ok(Some(raw_entry)) = seq.next_element::<RawCourseEntry>() {
            raw_entries.push(raw_entry)
        }

        let course_entries = CourseParser::new(raw_entries)
            .parse()
            .map_err(|e| de::Error::custom(e))?;

        Ok(course_entries)
    }

    // Code path for `Requirement`s that have a single JSON object in `course` field
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut url: Option<String> = None;
        let mut path: Option<String> = None;
        let mut guid: Option<GUID> = None;
        let mut name: Option<Option<String>> = None;
        let mut number: Option<Option<String>> = None;
        let mut subject_name: Option<Option<String>> = None;
        let mut subject_code: Option<Option<String>> = None;
        let mut credits: Option<(u8, Option<u8>)> = None;
        let mut is_narrative: Option<bool> = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            match key.as_str() {
                "url" => {
                    if url.is_some() {
                        return Err(de::Error::duplicate_field("url"));
                    }

                    url = Some(map.next_value()?);
                }
                "path" => {
                    if path.is_some() {
                        return Err(de::Error::duplicate_field("path"));
                    }

                    path = Some(map.next_value()?);
                }
                "guid" => {
                    if guid.is_some() {
                        return Err(de::Error::duplicate_field("guid"));
                    }

                    let guid_str_with_braces = map.next_value::<&str>()?;

                    let guid_str_trimmed = &guid_str_with_braces[1..guid_str_with_braces.len() - 1];

                    guid =
                        Some(GUID::try_from(guid_str_trimmed).map_err(|e| {
                            de::Error::custom(format!("error parsing guid: {}", e))
                        })?);
                }
                "name" => {
                    if name.is_some() {
                        return Err(de::Error::duplicate_field("name"));
                    }

                    name = Some(map.next_value()?);
                }
                "number" => {
                    if number.is_some() {
                        return Err(de::Error::duplicate_field("number"));
                    }

                    number = Some(map.next_value()?);
                }
                "subject_name" => {
                    if subject_name.is_some() {
                        return Err(de::Error::duplicate_field("subject_name"));
                    }

                    subject_name = Some(map.next_value()?);
                }
                "subject_code" => {
                    if subject_code.is_some() {
                        return Err(de::Error::duplicate_field("subject_code"));
                    }

                    subject_code = Some(map.next_value()?);
                }
                "credits" => {
                    if credits.is_some() {
                        return Err(de::Error::duplicate_field("credits"));
                    }

                    let credits_str = map.next_value::<&str>()?;
                    credits = Some(parse_course_credits(credits_str).map_err(de::Error::custom)?);
                }
                "is_narrative" => {
                    if is_narrative.is_some() {
                        return Err(de::Error::duplicate_field("is_narrative"));
                    }

                    let is_narrative_str = map.next_value::<&str>()?;

                    is_narrative = Some(match is_narrative_str {
                        "True" => true,
                        "False" => false,
                        invalid_str => {
                            return Err(de::Error::custom(format!(
                                r#"Expected "True" or "False". Got: {}"#,
                                invalid_str
                            )))
                        }
                    });
                }
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>();
                }
            }
        }

        let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
        let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
        let guid = guid.ok_or_else(|| de::Error::missing_field("guid"))?;
        let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
        let number = number.ok_or_else(|| de::Error::missing_field("number"))?;
        let subject_name = subject_name.ok_or_else(|| de::Error::missing_field("subject_name"))?;
        let subject_code = subject_code.ok_or_else(|| de::Error::missing_field("subject_code"))?;
        let credits = credits.ok_or_else(|| de::Error::missing_field("credits"))?;
        let is_narrative = is_narrative.ok_or_else(|| de::Error::missing_field("is_narrative"))?;

        let entry = if is_narrative {
            let name = name.ok_or(de::Error::custom(
                "`name` field for `Label` should not be null",
            ))?;
            CourseEntry::Label(Label {
                url,
                guid,
                name,
                subject_code,
                credits,
                number,
            })
        } else {
            let number = number.ok_or(de::Error::custom(
                "`number` field for `Course` should not be null",
            ))?;
            let subject_code = subject_code.ok_or(de::Error::custom(
                "`subject_code` field for `Course` should not be null",
            ))?;
            CourseEntry::Course(Course {
                url,
                path,
                guid,
                name,
                number,
                subject_name,
                subject_code,
                credits,
            })
        };

        Ok(CourseEntries(vec![entry]))
    }
}
