use deserialization::course::parse_course_credits;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;

use crate::{
    deserialization::course::{CourseParser, RawCourseEntry},
    guid::GUID,
};

mod deserialization;
pub mod guid;

/// Representation of a program in the catalog
#[derive(Debug, Deserialize)]
pub struct Program {
    /// Link to the official catalog
    pub url: String,

    /// GUID given by the system
    #[serde(deserialize_with = "deserialize_guid_with_curly_braces")]
    #[serde(rename = "GUID")]
    pub guid: GUID,

    /// Name of the program
    pub title: String,

    // TODO: Add `hours` field
    //
    /// Course requirements for the Program
    pub requirements: Option<Requirements>,
}

#[derive(Debug)]
pub enum Requirements {
    Single(RequirementModule),
    Many(Vec<RequirementModule>),
    /// Exists for in `Minor in Film Studies`
    SelectTrack,
}

#[derive(Debug)]
pub enum RequirementModule {
    SingleBasicRequirement {
        title: Option<String>,
        /// Originally `requirement_list` in the JSON payload
        requirement: Requirement,
    },
    /// The standard `RequirementModule` containing `Course`s
    BasicRequirements {
        title: Option<String>,
        requirements: Vec<Requirement>,
    },

    /// When told to "Select an emphasis below:". Ex: Major in Digital Media Communications
    SelectOneEmphasis { emphases: Vec<Requirement> },

    // SelectFromCourses {
    //     title: String,
    //     /// Number of courses or credits to select from the listed coureses
    //     /// TODO: Give a better name
    //     select_n: SelectN,
    //     req_narrative: Option<String>,
    //     selection: SelectionEntry,
    // },
    /// `RequirementModule`s where there is no `course` field in API JSON response
    Label { title: String },

    /// Variants that will be implemented in the future
    Unimplemented(Value),
}

// TODO: Extract all the useful information from the `req_narrative` field for each of the variants
// NOTE: The field `req_note` may contain useful information that can potentially be parsed
#[derive(Debug)]
pub enum Requirement {
    Courses {
        title: Option<String>,
        /// Originally `course` in the JSON payload:w
        entries: CourseEntries,
    },
    Select {
        entries: CourseEntries,
    },
    Label {
        title: Option<String>,
        req_narrative: Option<String>,
    },
}

#[derive(Debug)]
pub struct CourseEntries(Vec<CourseEntry>);

#[derive(Debug)]
pub enum CourseEntry {
    And(CourseEntries),
    Or(CourseEntries),
    Label(Label),
    Course(Course),
}

/// Representation of a course in the catalog
//
// NOTE: `Course` structs are normally deseriazed in a custom way through the `CourseEntries` struct to
// handle the potential operator entries (And, Or, etc) mixed within the array in the `course`
// field in JSON objects representing the `Requirement` struct. However, in special cases where the
// `course` field holds a JSON object representing a single `Course` struct, a different code path
// where the `Course` is separately deserialized into an intermediate struct, the private enum
// struct `RawRequirement` in the Deserialization implementation of the `Requirements` struct. The
// actual implementation of the special deserialization is in `CourseEntries` struct's
// `Deserialization` implementation where a sepcial `visit_map` is implemented for this used case
//
// TODO: Take account for labels at this level. Example in Bachelor of Music with Major in Composition
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Course {
    pub url: String,
    pub path: String,
    #[serde(deserialize_with = "deserialize_guid_with_curly_braces")]
    pub guid: GUID,
    pub name: String,
    pub number: String,
    pub subject_name: Option<String>,
    pub subject_code: String,

    /// The representation of possible credits earned by completing the course. The lower bound is
    /// the minimum that you can earn while the upper bound is the max. If there is a max, then the
    /// tuple should be interpreted as an inclusive range from the lower bound to the upper bound,
    /// which can be think of as (lower bound..=upper bound).
    pub credits: (u8, Option<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    url: String,
    guid: GUID,
    name: String,
    subject_code: Option<String>,
    credits: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SelectN {
    pub n: u8,
    pub unit: CourseSelectionUnit,
}

#[derive(Debug, Clone, Deserialize)]
pub enum CourseSelectionUnit {
    Credits,
    Courses,
}

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

fn deserialize_guid_with_curly_braces<'de, D>(de: D) -> Result<GUID, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: &str = Deserialize::deserialize(de)?;

    // Ommit the curly braces in the source when parsing
    s = &s[1..s.len() - 1];

    GUID::try_from(s).map_err(|e| serde::de::Error::custom(e))
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
        println!("visit_map in RequirementVisitor");

        let mut title = None;
        let mut req_narrative: Option<Option<String>> = None;
        let mut courses = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            dbg!(&key);
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

        dbg!(&title);
        dbg!(&req_narrative);
        dbg!(&courses);

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

        dbg!(&requirement);

        Ok(requirement)
    }
}

impl<'de> Deserialize<'de> for CourseEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        println!("hello from CourseEntriesVisitor");
        deserializer.deserialize_any(CourseEntriesVisitor)
    }
}

struct CourseEntriesVisitor;

impl<'de> Visitor<'de> for CourseEntriesVisitor {
    type Value = CourseEntries;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of JSON objects representing a `SelectionEntry`")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        println!("visit_map in CourseEntriesVisitor");

        let mut url: Option<String> = None;
        let mut path: Option<String> = None;
        let mut guid: Option<GUID> = None;
        let mut name: Option<String> = None;
        let mut number: Option<String> = None;
        let mut subject_name: Option<Option<String>> = None;
        let mut subject_code: Option<String> = None;
        let mut credits: Option<(u8, Option<u8>)> = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            dbg!(&key);

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

                    number = Some(map.next_value::<String>()?);
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
                _ => {
                    let _ = map.next_value::<de::IgnoredAny>();
                }
            }
        }

        dbg!(&url);
        dbg!(&path);
        dbg!(&guid);
        dbg!(&name);
        dbg!(&number);
        dbg!(&subject_name);
        dbg!(&subject_code);
        dbg!(&credits);

        let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
        let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
        let guid = guid.ok_or_else(|| de::Error::missing_field("guid"))?;
        let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
        let number = number.ok_or_else(|| de::Error::missing_field("number"))?;
        let subject_name = subject_name.ok_or_else(|| de::Error::missing_field("subject_name"))?;
        let subject_code = subject_code.ok_or_else(|| de::Error::missing_field("subject_code"))?;
        let credits = credits.ok_or_else(|| de::Error::missing_field("credits"))?;

        let entry = CourseEntry::Course(Course {
            url,
            path,
            guid,
            name,
            number,
            subject_name,
            subject_code,
            credits,
        });

        Ok(CourseEntries(vec![entry]))
    }

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
}

#[cfg(test)]
mod test {
    use core::panic;

    use super::*;

    #[test]
    fn can_parse_program_with_a_single_basic_requirement() {
        let program_json = std::fs::read_to_string("./data/cs_major.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        let expected_url = "https://iq5prod1.smartcatalogiq.com:443/en/catalogs/union-university/2023/academic-catalogue-undergraduate-catalogue/college-of-arts-and-sciences/department-of-computer-science/major-in-computer-science-42-hours";
        let expected_guid = GUID::try_from("5B72AC3A-9A84-4CF5-B1BE-B3E0B48163A5").unwrap();
        let expected_title = "Major in Computer Science—42 hours";

        let expected_req_mod_title = "Degree Requirements";

        assert_eq!(parsed_program.url, expected_url);
        assert_eq!(parsed_program.guid, expected_guid);
        assert_eq!(parsed_program.title, expected_title);

        assert!(parsed_program.requirements.is_some());

        if let Some(Requirements::Single(req_mod)) = parsed_program.requirements {
            if let RequirementModule::BasicRequirements {
                title,
                requirements: _,
            } = req_mod
            {
                assert_eq!(title.unwrap().as_str(), expected_req_mod_title);
            } else {
                panic!("Expected requirement_module to be the `BasicRequirements` variant");
            }
        } else {
            panic!("Expected requirements to be the `Single` variant")
        }
    }

    #[test]
    fn can_parse_program_with_many_basic_requirements() {
        let program_json = std::fs::read_to_string("./data/digital_media_major.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        let expected_url = "https://iq5prod1.smartcatalogiq.com:443/en/catalogs/union-university/2023/academic-catalogue-undergraduate-catalogue/college-of-arts-and-sciences/department-of-communication-arts/major-in-digital-media-communications-48-hours";
        let expected_guid = GUID::try_from("0780CBF3-68C6-4999-95B9-7722170F47DD").unwrap();
        let expected_title = "Major in Digital Media Communications—48 hours";

        assert_eq!(parsed_program.url, expected_url);
        assert_eq!(parsed_program.guid, expected_guid);
        assert_eq!(parsed_program.title, expected_title);

        assert!(parsed_program.requirements.is_some());

        if let Some(Requirements::Many(req_mods)) = parsed_program.requirements {
            // TODO: Check the sub types for equivalence
            assert_eq!(req_mods.len(), 2);
        } else {
            panic!("Expected requirements to be the `Many` variant")
        }
    }

    #[test]
    fn can_parse_program_with_requirement_having_a_single_course() {
        let program_json = std::fs::read_to_string("./data/zoology_major.json").unwrap();
        let parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");

        let req_mod = if let Some(Requirements::Single(req_mod)) = parsed_program.requirements {
            req_mod
        } else {
            panic!("Expected requirements to be the `Many` variant")
        };

        let requirements = if let RequirementModule::BasicRequirements {
            title,
            requirements,
        } = req_mod
        {
            assert_eq!(title.unwrap().as_str(), "Degree Requirements");
            assert_eq!(requirements.len(), 4);
            requirements
        } else {
            panic!("Expected `RequirementModule` to be the `BasicRequirements` variant");
        };

        if let Requirement::Courses { title, entries } = &requirements[0] {
            assert_eq!(
                title.as_ref().unwrap().as_str(),
                "Prerequisite/Corequisite:"
            );
            assert_eq!(entries.0.len(), 1);
        } else {
            panic!(
                "Expected `Requirement` to be the `Courses` variant. Got: {:?}",
                requirements[0]
            );
        }
    }
}
