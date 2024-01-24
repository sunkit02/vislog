# VISLOG (Catalog Visualization)

Vislog is a project to visualize the Union University Catalog

## Architecture

![Image of Overall Architecture](./docs/vislog_architecture.png)


### Backend

#### JSON Processing

![Image of Course Parsing State Machine](./docs/course_parsing_state_machine.svg)

- Manual trigger / API endpoint
- Gets triggered annually
- Manual trigger / timer

#### Static Asset Server

- Home page
- Program graphs and course relationships

#### Static Asset Storage

- Parsed schema
- Generated program visualizations

#### Interactive Server

- Maintain web socket connections between sessions
- Reads program information from the `static asset server`
- Authentication with Union's SSO / Azure Active Directory

#### User Data Storage

- User data
- Student id for identification
- Could be blob storage storing user data as files
- JSON or binary files


### Frontend

#### Static Site

- Display home page
- Display generated program visualizations
- Combine with interactive client?
    - Can reduce system complexity
    - Combines static server with interactive server (Seperation of concerns?)

#### Interactive Clients

- Interactive planning for program progression
- Planning by semesters
- Real time collaboration between student and advisor
- Show sessions of courses available? (Depending on the data provided by IT)
- Search bar for allowing to arbitrarily select courses not already selected in
  program map to compensate for the parts of the catalog that cannot be parsed
