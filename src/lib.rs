use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;

use crate::guid::GUID;

pub mod guid;

/// Representation of a program in the catalog
#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone)]
pub enum Requirements {
    Single(RequirementModule),
    Many(Vec<RequirementModule>),
    /// Exists for in `Minor in Film Studies`
    SelectTrack,
}

#[derive(Debug, Clone)]
pub enum RequirementModule {
    /// The standard `RequirementModule` containing `Course`s
    BasicRequirements {
        title: String,
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

#[derive(Debug, Clone)]
pub enum Requirement {
    Courses { title: String, courses: Vec<Course> },
    Select { n: u8, options: SelectionEntry },
}

#[derive(Debug, Clone, Deserialize)]
pub enum SelectionEntry {
    And {
        lhs: Box<SelectionEntry>,
        rhs: Box<SelectionEntry>,
    },
    Or(Vec<SelectionEntry>),
    Courses(Vec<Course>),
}

/// Select *N* credits/courses from the following [Courses](crate::Course)
// WARN: Not used yet
#[derive(Debug, Clone, Deserialize)]
pub struct SelectFrom {
    /// The number of courses to select from
    /// NOTE: Can be both an integer or a number of credits
    pub n: usize,

    /// The courses to select from
    pub courses: Vec<Course>,
}

/// Representation of a course in the catalog
// TODO: Take account for labels at this level. Example in Bachelor of Music with Major in Composition
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Course {
    pub url: String,
    pub path: String,
    #[serde(deserialize_with = "deserialize_guid_with_curly_braces")]
    pub guid: GUID,
    pub name: String,
    // TODO: Make Option<u16>,
    pub number: Option<String>,
    pub subject_name: Option<String>,
    pub subject_code: Option<String>,
    // TODO: Make Option<u8>,
    pub credits: Option<String>,
    // TODO: Make Option<bool>,
    pub is_narrative: Option<String>,
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
        let mut title: Option<String> = None;
        let mut req_narrative: Option<Option<String>> = None;
        let mut requirements: Option<Vec<Requirement>> = None;

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
                    if requirements.is_some() {
                        return Err(de::Error::duplicate_field("requirements"));
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

        let requirement_module = RequirementModule::BasicRequirements {
            title,
            requirements,
        };

        Ok(Requirements::Single(requirement_module))
    }

    /// Case for [Requirements::Many] variant
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        println!("seq visited");

        let mut modules = Vec::new();
        while let Ok(Some(module)) = seq.next_element() {
            println!("Hello");
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
        let mut title: Option<String> = None;
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
        let mut courses = None;

        while let Ok(Some(key)) = map.next_key::<String>() {
            match key.as_str() {
                "title" => {
                    if title.is_some() {
                        return Err(de::Error::duplicate_field("title"));
                    }

                    title = Some(map.next_value()?);
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
        let courses = courses.ok_or_else(|| de::Error::missing_field("course"))?;

        Ok(Requirement::Courses { title, courses })
    }
}

impl<'de> Deserialize<'de> for Requirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(RequirementVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn can_deserialize_guid_with_curly_braces() {
        // Parsing course that contains the field `guid` being deserialized by
        // `deserialize_guid_with_curly_braces`
        let course_json = r#"{
            "url": "http://foo.com/bar/baz",
            "path": "/foo/bar",
            "guid": "{81A2EE85-6A90-49FB-A38A-B63481C8E123}",
            "name": "Foo",
            "number": "123",
            "subject_name": "Testing",
            "subject_code": "TST",
            "credits": "2",
            "is_narrative": "False"
        }"#;

        let parsed_course = serde_json::from_str::<Course>(course_json).unwrap();

        let expected = Course {
            url: "http://foo.com/bar/baz".to_owned(),
            path: "/foo/bar".to_owned(),
            guid: GUID::try_from("81A2EE85-6A90-49FB-A38A-B63481C8E123")
                .expect("Failed to parse GUID"),
            name: "Foo".to_owned(),
            number: Some("123".to_owned()),
            subject_name: Some("Testing".to_owned()),
            subject_code: Some("TST".to_owned()),
            credits: Some("2".to_owned()),
            is_narrative: Some("False".to_owned()),
        };

        assert_eq!(parsed_course, expected);
    }

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
                requirements,
            } = req_mod
            {
                assert_eq!(title, expected_req_mod_title);
                // assert_eq!(requirements[0].title, "Prerequisites:");
                // assert_eq!(
                //     requirements[0]
                //         .courses
                //         .as_ref()
                //         .expect("Should have courses")
                //         .len(),
                //     4
                // );

                // assert_eq!(requirements[1].title, "Major Requirements:");
                // assert_eq!(
                //     requirements[1]
                //         .courses
                //         .as_ref()
                //         .expect("Should have courses")
                //         .len(),
                //     16
                // );
            } else {
                panic!("Expected requirement_module to be the `BasicRequirements` variant");
            }
        } else {
            panic!("Expected requirements to be the `Single` variant")
        }
    }

    #[test]
    #[ignore]
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
}
