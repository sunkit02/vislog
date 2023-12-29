use serde::{Deserialize, Deserializer};

use crate::guid::GUID;

pub mod guid;

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
    // /// Course requirements for the Program
    // pub requirements: Option<Requirements>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Requirements {
    Single(RequirementModule),
    Many(Vec<RequirementModule>),
}

#[derive(Debug, Clone, Deserialize)]
pub enum RequirementModule {
    BasicRequirements {
        title: String,
        req_narrative: Option<String>,
        requirements: Vec<Requirement>,
    },
    SelectFromCourses {
        title: String,
        /// Number of courses or credits to select from the listed coureses
        /// TODO: Give a better name
        select_n: SelectN,
        req_narrative: Option<String>,
        courses: Vec<Course>,
    },
    Unimplemented,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Requirement {
    pub title: String,

    #[serde(rename = "course")]
    pub courses: Vec<Course>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Or {}

#[derive(Debug, Clone, Deserialize)]
pub struct And {}

/// Select *N* credits/courses from the following [Courses](crate::Course)
#[derive(Debug, Clone, Deserialize)]
pub struct SelectFrom {
    /// The number of courses to select from
    /// NOTE: Can be both an integer or a number of credits
    pub n: usize,

    /// The courses to select from
    pub courses: Vec<Course>,
}

// TODO: becomes enum for both courses and operators
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Course {
    pub url: String,
    pub path: String,
    #[serde(deserialize_with = "deserialize_guid_with_curly_braces")]
    pub guid: GUID,
    pub name: String,
    // pub number: Option<u16>,
    pub number: Option<String>,
    pub subject_name: Option<String>,
    pub subject_code: Option<String>,
    // pub credits: Option<u8>,
    pub credits: Option<String>,
    // pub is_narrative: Option<bool>,
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

// impl<'de> Deserialize<'de> for Program {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         todo!()
//     }
// }
//
// struct ProgramVisitor;
//
// impl<'de> Visitor<'de> for ProgramVisitor {
//     type Value = Program;
//
//     fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
//         // TODO: Improve this message
//         formatter.write_str("a JSON object representing a program at Union University")
//     }
//
//     fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
//     where
//         A: serde::de::MapAccess<'de>,
//     {
//         let mut url: Option<String> = None;
//         let mut guid_str: Option<String> = None;
//         let mut title: Option<String> = None;
//         let mut requirements: Option<Option<Requirements>> = None;
//
//         while let Ok(Some(key)) = map.next_key::<String>() {
//             match key.as_str() {
//                 "url" => {
//                     if url.is_some() {
//                         return Err(de::Error::duplicate_field("url"));
//                     }
//
//                     url = Some(map.next_value()?);
//                 }
//                 "guid" => {
//                     if guid_str.is_some() {
//                         return Err(de::Error::duplicate_field("guid"));
//                     }
//                     guid_str = Some(map.next_value()?);
//                 }
//                 "title" => {
//                     if title.is_some() {
//                         return Err(de::Error::duplicate_field("title"));
//                     }
//                     title = Some(map.next_value()?);
//                 }
//                 "requirements" => {
//                     if requirements.is_some() {
//                         return Err(de::Error::duplicate_field("requirements"));
//                     }
//                     requirements = Some(map.next_value()?);
//                 }
//                 _ => {
//                     let _ = map.next_value::<de::IgnoredAny>();
//                 }
//             }
//         }
//
//         let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
//         let guid_str = guid_str.ok_or_else(|| de::Error::missing_field("guid"))?;
//         let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
//         let requirements = requirements.ok_or_else(|| de::Error::missing_field("requirements"))?;
//
//         let guid_str = &guid_str[1..guid_str.len() - 1];
//         let guid = GUID::try_from(guid_str).map_err(|e| serde::de::Error::custom(e.to_string()))?;
//
//         Ok(Program {
//             url,
//             guid,
//             title,
//             requirements,
//         })
//     }
// }

fn deserialize_guid_with_curly_braces<'de, D>(de: D) -> Result<GUID, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: &str = Deserialize::deserialize(de)?;

    // Ommit the curly braces in the source when parsing
    s = &s[1..s.len() - 1];

    GUID::try_from(s).map_err(|e| serde::de::Error::custom(e))
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
    fn can_parse_cs_major() {
        let cs_major_json = std::fs::read_to_string("./data/cs_major.json").unwrap();

        let parsed_cs_major = serde_json::from_str::<Program>(cs_major_json.as_str()).unwrap();

        let expected_url = "https://iq5prod1.smartcatalogiq.com:443/en/catalogs/union-university/2023/academic-catalogue-undergraduate-catalogue/college-of-arts-and-sciences/department-of-computer-science/major-in-computer-science-42-hours";
        let expected_guid = GUID::try_from("5B72AC3A-9A84-4CF5-B1BE-B3E0B48163A5").unwrap();
        let expected_title = "Major in Computer Science—42 hours";

        assert_eq!(parsed_cs_major.url, expected_url);
        assert_eq!(parsed_cs_major.guid, expected_guid);
        assert_eq!(parsed_cs_major.title, expected_title);
    }
}
