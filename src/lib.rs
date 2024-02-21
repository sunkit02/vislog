use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::parsing::guid::{deserialize_guid_with_curly_braces, GUID};

pub mod parsing;

/// Representation of a program in the catalog
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
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

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Requirements {
    Single(RequirementModule),
    Many(Vec<RequirementModule>),
    /// Exists for in `Minor in Film Studies`
    SelectTrack,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data")]
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
    //
    // TODO: Finish this
    SelectOneEmphasis { emphases: Vec<Requirement> },

    /// `RequirementModule`s where there is no `course` field in API JSON response
    Label { title: String },

    /// Variants that will be implemented in the future
    Unimplemented(Value),
}

// TODO: Extract all the useful information from the `req_narrative` field for each of the variants
// NOTE: The field `req_note` may contain useful information that can potentially be parsed
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Requirement {
    Courses {
        title: Option<String>,
        /// Originally `course` in the JSON payload:w
        entries: CourseEntries,
    },
    SelectFromCourses {
        title: String,
        // TODO: Add the `num_to_select` and `selection_unit` fields
        // num_to_select: u8,
        // selection_unit: CourseUnit,
        courses: Option<CourseEntries>,
    },
    Label {
        title: Option<String>,
        req_narrative: Option<String>,
    },
}

#[derive(Debug)]
pub enum CourseUnit {
    Course,
    Hours,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CourseEntries(Vec<CourseEntry>);

impl Deref for CourseEntries {
    type Target = Vec<CourseEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CourseEntries {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "data")]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Course {
    pub url: String,
    pub path: String,
    #[serde(deserialize_with = "deserialize_guid_with_curly_braces")]
    pub guid: GUID,

    /// This field is normally not, but sometimes can be empty for special courses.
    ///
    /// ### Examples
    /// Such examples can be found in the "Applied Studies" `Requirement` for [Bachelor of Music with Major in Worship Leadership](https://iq5prod1.smartcatalogiq.com/en/catalogs/union-university/2023/academic-catalogue-undergraduate-catalogue/college-of-arts-and-sciences/department-of-music/bachelor-of-music-with-major-in-worship-leadership-83-hours-36-hours-47-hour-worship-leadership-core)
    ///
    /// #### Ex: Applied Studies-12 hours:
    /// | Course  |     Name     |     Credits     |
    /// |---------|--------------|-----------------|
    /// | MUS 150 | <empty-name> | <empty-credits> |
    /// | MUS 150 | <empty-name> | <empty-credits> |
    /// | MUS 250 | <empty-name> | <empty-credits> |
    /// | MUS 250 | <empty-name> | <empty-credits> |
    /// | MUS 350 | <empty-name> | <empty-credits> |
    /// | MUS 350 | <empty-name> | <empty-credits> |
    /// | MUS 450 | <empty-name> | <empty-credits> |
    ///
    // NOTE: In the "Applied Studies" example all the courses had the field `is_narrative` set as
    // "True" which may be useful in the future
    pub name: Option<String>,
    pub number: String,
    pub subject_name: Option<String>,
    pub subject_code: String,

    /// The representation of possible credits earned by completing the course. The lower bound is
    /// the minimum that you can earn while the upper bound is the max. If there is a max, then the
    /// tuple should be interpreted as an inclusive range from the lower bound to the upper bound,
    /// which can be think of as (lower bound..=upper bound).
    pub credits: (u8, Option<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Label {
    pub url: String,
    pub guid: GUID,
    pub name: String,
    pub number: Option<String>,
    pub subject_code: Option<String>,
    pub credits: (u8, Option<u8>),
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
            assert_eq!(entries.len(), 1);
        } else {
            panic!(
                "Expected `Requirement` to be the `Courses` variant. Got: {:?}",
                requirements[0]
            );
        }
    }

    #[test]
    #[ignore = "fix this mystery later"]
    fn can_parse_program_claiming_to_have_trailing_characters() {
        let program_json = std::fs::read_to_string("./data/family_studies_major.json").unwrap();
        let _parsed_program = serde_json::from_str::<Program>(program_json.as_str())
            .expect("Failed to parse `Program`");
    }
}
