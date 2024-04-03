use serde_json;
use vislog::{Course, CourseEntries, Label, Program, Requirement};

fn main() {
    let program_json = std::fs::read_to_string("./data/cs_major.json").unwrap();

    let cs_major: Program = serde_json::from_str(&program_json).unwrap();

    // println!("Program Name: {}", cs_major.title);

    let _requirements = cs_major
        .requirements
        .as_ref()
        .map(|reqs| {
            let mut req_mods = Vec::new();
            match reqs {
                vislog::Requirements::Single(module) => req_mods.push(module),
                vislog::Requirements::Many(mods) => req_mods.extend(mods),
                vislog::Requirements::SelectTrack => todo!(),
            }
            req_mods
        })
        .map(|req_mods| {
            req_mods
                .iter()
                .map(|m| match m {
                    vislog::RequirementModule::SingleBasicRequirement { requirement, .. } => {
                        vec![get_req_title(&requirement)]
                    }
                    vislog::RequirementModule::BasicRequirements { requirements, .. } => {
                        requirements.iter().map(get_req_title).collect()
                    }
                    vislog::RequirementModule::SelectOneEmphasis { .. } => todo!(),
                    vislog::RequirementModule::Label { title } => vec![Some(title.as_str())],
                    vislog::RequirementModule::Unimplemented(_) => todo!(),
                })
                .flatten()
                .flatten()
                .collect::<Vec<_>>()
        });
    // println!("Requirements: {:?}", requirements);

    let _courses_titles = cs_major
        .requirements
        .as_ref()
        .map(|reqs| {
            let mut req_mods = Vec::new();
            match reqs {
                vislog::Requirements::Single(module) => req_mods.push(module),
                vislog::Requirements::Many(mods) => req_mods.extend(mods),
                vislog::Requirements::SelectTrack => todo!(),
            }
            req_mods
        })
        .map(|req_mods| {
            req_mods
                .iter()
                .map(|m| match m {
                    vislog::RequirementModule::SingleBasicRequirement { requirement, .. } => {
                        get_req_courses_titles(&requirement)
                    }
                    vislog::RequirementModule::BasicRequirements { requirements, .. } => {
                        requirements
                            .iter()
                            .map(get_req_courses_titles)
                            .flatten()
                            .collect()
                    }
                    vislog::RequirementModule::SelectOneEmphasis { .. } => todo!(),
                    vislog::RequirementModule::Label { title } => vec![title.as_str()],
                    vislog::RequirementModule::Unimplemented(_) => todo!(),
                })
                .collect::<Vec<_>>()
        });

    // println!("Courses: {courses_titles:#?}");

    println!("{}", serde_json::to_string_pretty(&cs_major).unwrap())
}

fn get_req_title(req: &Requirement) -> Option<&str> {
    match req {
        Requirement::Courses { title, .. } => title.as_ref().map(|s| s.as_str()),
        Requirement::SelectFromCourses { title, .. } => Some(title.as_str()),
        Requirement::Label { title, .. } => title.as_ref().map(|s| s.as_str()),
    }
}

fn get_req_courses_titles(req: &Requirement) -> Vec<&str> {
    fn extract_course_titles(entries: &CourseEntries) -> Vec<&str> {
        entries
            .iter()
            .filter_map(|entry| match entry {
                vislog::CourseEntry::And(entries) => Some(extract_course_titles(entries)),
                vislog::CourseEntry::Or(entries) => Some(extract_course_titles(entries)),
                vislog::CourseEntry::Label(Label { name, .. }) => Some(vec![name.as_str()]),
                vislog::CourseEntry::Course(Course { name, .. }) => {
                    name.as_ref().map(|n| vec![n.as_str()])
                }
            })
            .flatten()
            .collect()
    }

    match req {
        Requirement::Courses { courses, .. } => extract_course_titles(courses),
        Requirement::SelectFromCourses { courses, .. } => courses
            .as_ref()
            .map(extract_course_titles)
            .unwrap_or(Vec::new()),
        Requirement::Label { title, .. } => title
            .as_ref()
            .map(|t| vec![t.as_str()])
            .unwrap_or(Vec::new()),
    }
}
