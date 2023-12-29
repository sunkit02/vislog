use serde::Deserialize;

use crate::guid::GUID;

pub mod guid;

#[derive(Debug, Clone, Deserialize)]
pub struct Program {
    /// Link to the official catalog
    pub url: String,

    /// GUID given by the system
    #[serde(alias = "GUID")]
    pub guid: String,

    /// Name of the program
    pub title: String,

    /// Course requirements for the Program
    pub requirements: Option<Requirements>,
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
#[derive(Debug, Clone, Deserialize)]
pub struct Course {
    pub path: String,
    pub url: String,
    pub guid: GUID,
    pub name: String,
    pub number: Option<u16>,
    pub subject_name: Option<String>,
    pub subject_code: Option<String>,
    pub credits: Option<u8>,
    pub is_narrative: Option<bool>,
}
