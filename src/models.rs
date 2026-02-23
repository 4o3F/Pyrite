use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Duration, FixedOffset};
use serde::{self, Deserialize, Deserializer, Serialize};
use tracing::error;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EventType {
    #[serde(rename = "contest")]
    Contest,
    #[serde(rename = "judgement-types")]
    JudgementTypes,
    #[serde(rename = "languages")]
    Languages,
    #[serde(rename = "problems")]
    Problems,
    #[serde(rename = "groups")]
    Groups,
    #[serde(rename = "organizations")]
    Organizations,
    #[serde(rename = "teams")]
    Teams,
    #[serde(rename = "persons")]
    Persons,
    #[serde(rename = "accounts")]
    Accounts,
    #[serde(rename = "state")]
    State,
    #[serde(rename = "submissions")]
    Submissions,
    #[serde(rename = "judgements")]
    Judgements,
    #[serde(rename = "runs")]
    Runs,
    #[serde(rename = "clarifications")]
    Clarifications,
    #[serde(rename = "awards")]
    Awards,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub token: Option<String>,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub data: Option<serde_json::Value>,
    pub time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JudgementType {
    pub id: String,
    pub name: String,
    pub penalty: bool,
    pub solved: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub id: String,
    pub hidden: bool,
    pub icpc_id: Option<String>,
    pub name: String,
    pub sortorder: i32,
    pub color: Option<String>,
    pub allow_self_registration: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Organization {
    pub id: String,
    pub icpc_id: Option<String>,
    pub name: String,
    pub formal_name: String,
    pub shortname: String,
    pub country: String,

    #[serde(default)]
    pub logo: Vec<OrganizationImage>,

    #[serde(rename = "country_flag", default)]
    pub country_flags: Vec<OrganizationImage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrganizationImage {
    pub href: String,
    pub mime: String,
    pub filename: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Team {
    pub location: Option<Location>,
    pub organization_id: Option<String>,
    pub hidden: bool,
    pub group_ids: Vec<String>,
    pub affiliation: Option<String>,
    pub nationality: Option<String>,
    pub id: String,
    pub icpc_id: Option<String>,
    pub label: Option<String>,
    pub name: String,
    pub display_name: Option<String>,
    pub public_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub name: String,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub last_login_time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub last_api_login_time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub first_login_time: Option<DateTime<FixedOffset>>,
    pub team: String,
    pub team_id: String,
    pub roles: Vec<String>,
    pub r#type: String,
    pub email: Option<String>,
    pub last_ip: String,
    pub ip: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Problem {
    pub ordinal: i32,
    pub id: String,
    #[serde(rename = "short_name")]
    pub short_name: String,
    pub rgb: String,
    pub color: String,
    pub label: String,
    #[serde(rename = "time_limit")]
    pub time_limit: f64,
    pub statement: Vec<serde_json::Value>,
    #[serde(rename = "externalid")]
    pub external_id: Option<String>,
    pub name: String,
    #[serde(rename = "test_data_count")]
    pub test_data_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Submission {
    pub language_id: String,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_duration_str")]
    pub contest_time: Duration,
    pub team_id: String,
    pub problem_id: String,
    pub files: Vec<SubmissionFile>,
    pub id: String,
    pub external_id: Option<String>,
    pub entry_point: Option<String>,
    pub import_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubmissionFile {
    pub href: String,
    pub mime: String,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Judgement {
    pub max_run_time: Option<f64>,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub start_time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_duration_str")]
    pub start_contest_time: Duration,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub end_time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_opt_duration_str")]
    pub end_contest_time: Option<Duration>,
    pub submission_id: String,
    pub id: String,
    pub valid: bool,
    pub judgement_type_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Award {
    pub id: String,
    pub citation: String,
    pub team_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contest {
    pub formal_name: String,
    pub scoreboard_type: String,

    #[serde(deserialize_with = "from_opt_datetime")]
    pub start_time: Option<DateTime<FixedOffset>>,
    #[serde(deserialize_with = "from_opt_datetime")]
    pub end_time: Option<DateTime<FixedOffset>>,
    /// Scoreboard unfrozen time
    #[serde(deserialize_with = "from_opt_datetime")]
    pub scoreboard_thaw_time: Option<DateTime<FixedOffset>>,

    #[serde(deserialize_with = "from_duration_str")]
    pub duration: Duration,
    #[serde(deserialize_with = "from_duration_str")]
    pub scoreboard_freeze_duration: Duration,
    pub id: String,
    pub external_id: Option<String>,
    pub name: String,
    pub shortname: String,
    pub allow_submit: bool,
    pub runtime_as_score_tiebreaker: bool,
    pub warning_message: Option<String>,
    pub penalty_time: i32,

    // Helper field from now on
    #[serde(skip_deserializing)]
    pub scoreboard_freeze_time: Option<DateTime<FixedOffset>>,
}

#[derive(Debug)]
pub struct ContestState {
    pub contest: Option<Contest>,
    pub judgement_types: HashMap<String, JudgementType>,
    pub groups: HashMap<String, Group>,
    pub organizations: HashMap<String, Organization>,
    pub teams: HashMap<String, Team>,
    pub accounts: HashMap<String, Account>,
    pub problems: HashMap<String, Problem>,
    pub submissions: HashMap<String, Submission>,
    pub judgements: HashMap<String, Judgement>,
    pub awards: HashMap<String, Award>,
    pub leaderboard_pre_freeze: Vec<TeamStatus>,
    pub leaderboard_finalized: Vec<TeamStatus>,
}

impl ContestState {
    pub fn new() -> Self {
        ContestState {
            contest: None,
            judgement_types: HashMap::new(),
            groups: HashMap::new(),
            organizations: HashMap::new(),
            teams: HashMap::new(),
            accounts: HashMap::new(),
            problems: HashMap::new(),
            submissions: HashMap::new(),
            judgements: HashMap::new(),
            awards: HashMap::new(),
            leaderboard_pre_freeze: Vec::new(),
            leaderboard_finalized: Vec::new(),
        }
    }
}

fn from_opt_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<FixedOffset>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(s) = opt {
        let dt = DateTime::parse_from_rfc3339(&s).map_err(serde::de::Error::custom)?;
        Ok(Some(dt))
    } else {
        Ok(None)
    }
}

fn from_duration_str<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let negative = s.starts_with('-');
    let trimmed = s.trim_start_matches('-');
    let parts: Vec<&str> = trimmed.split(':').collect();
    if parts.len() != 3 {
        return Err(serde::de::Error::custom(format!(
            "invalid duration format: {}",
            s
        )));
    }

    let hours: i64 = parts[0].parse().map_err(serde::de::Error::custom)?;
    let minutes: i64 = parts[1].parse().map_err(serde::de::Error::custom)?;
    let seconds: f64 = parts[2].parse().map_err(serde::de::Error::custom)?;

    let total_secs = (hours * 3600 + minutes * 60) + seconds as i64;
    Ok(if negative {
        -Duration::seconds(total_secs)
    } else {
        Duration::seconds(total_secs)
    })
}

fn from_opt_duration_str<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    if let Some(s) = opt {
        let negative = s.starts_with('-');
        let trimmed = s.trim_start_matches('-');
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() != 3 {
            return Err(serde::de::Error::custom(format!(
                "invalid duration format: {}",
                s
            )));
        }

        let hours: i64 = parts[0].parse().map_err(serde::de::Error::custom)?;
        let minutes: i64 = parts[1].parse().map_err(serde::de::Error::custom)?;
        let seconds: f64 = parts[2].parse().map_err(serde::de::Error::custom)?;

        let total_secs = (hours * 3600 + minutes * 60) + seconds as i64;
        Ok(Some(if negative {
            -Duration::seconds(total_secs)
        } else {
            Duration::seconds(total_secs)
        }))
    } else {
        Ok(None)
    }
}

pub trait HasId {
    fn id(&self) -> &str;
}

impl HasId for JudgementType {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Group {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Organization {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Team {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Account {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Problem {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Submission {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Judgement {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Award {
    fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStatus {
    pub team_id: String,
    pub team_name: String,
    pub team_affiliation: String,
    pub sortorder: i32,
    pub total_points: i32,
    pub total_penalty: i64,
    pub last_ac_time: Option<DateTime<FixedOffset>>,
    pub problem_stats: HashMap<String, ProblemStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemStat {
    pub solved: bool,
    /// If attempted_during_freeze is false, then there's no submission during freeze
    pub attempted_during_freeze: bool,
    pub penalty: i64,
    pub submissions_before_solved: i32,
    pub first_ac_time: Option<DateTime<FixedOffset>>,
}

impl TeamStatus {
    pub fn new(
        team_id: String,
        team_name: String,
        team_affiliation: String,
        sortorder: i32,
    ) -> Self {
        Self {
            team_id,
            team_name,
            team_affiliation,
            sortorder,
            total_points: 0,
            total_penalty: 0,
            last_ac_time: None,
            problem_stats: HashMap::new(),
        }
    }

    pub fn add_submission(
        &mut self,
        problem_id: &str,
        submission_time: DateTime<FixedOffset>,
        judgement_type_id: Option<&str>,
        judgement_types: &HashMap<String, JudgementType>,
        contest_start_time: Option<DateTime<FixedOffset>>,
        contest_freeze_time: Option<DateTime<FixedOffset>>,
    ) {
        let problem_stat =
            self.problem_stats
                .entry(problem_id.to_string())
                .or_insert(ProblemStat {
                    solved: false,
                    attempted_during_freeze: false,
                    penalty: 0,
                    submissions_before_solved: 0,
                    first_ac_time: None,
                });

        if problem_stat.solved {
            return;
        }

        if let Some(judgement_type_id) = judgement_type_id
            && let Some(judgement_type) = judgement_types.get(judgement_type_id)
        {
            if judgement_type.penalty || judgement_type.solved {
                problem_stat.submissions_before_solved += 1;
            }

            problem_stat.attempted_during_freeze =
                if let Some(contest_freeze_time) = contest_freeze_time {
                    submission_time > contest_freeze_time
                } else {
                    error!("No contest freeze time specified!");
                    unreachable!()
                };

            if judgement_type.solved {
                problem_stat.solved = true;
                problem_stat.first_ac_time = Some(submission_time);

                let contest_time = if let Some(start_time) = contest_start_time {
                    submission_time - start_time
                } else {
                    error!("No contest start time specified!");
                    return;
                };

                let penalty_minutes = (problem_stat.submissions_before_solved - 1) * 20;
                let problem_penalty = contest_time.num_minutes() + penalty_minutes as i64;
                problem_stat.penalty = problem_penalty;

                if problem_stat.attempted_during_freeze {
                    // If solved happen during scoreboard freeze, we don't add penalty yet, wait for scoreboard roll
                    return;
                }

                self.total_points += 1;
                self.total_penalty += problem_penalty;
                if self.last_ac_time.is_none_or(|last| submission_time > last) {
                    self.last_ac_time = Some(submission_time);
                }
            }
        }
    }
}

impl PartialEq for TeamStatus {
    fn eq(&self, other: &Self) -> bool {
        self.team_id == other.team_id
    }
}

impl Eq for TeamStatus {}

impl PartialOrd for TeamStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TeamStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by sortorder
        if self.sortorder != other.sortorder {
            return self.sortorder.cmp(&other.sortorder);
        }
        // Sort by solved problem score
        if self.total_points != other.total_points {
            return other.total_points.cmp(&self.total_points);
        }
        // Sort by penalty time
        if self.total_penalty != other.total_penalty {
            return self.total_penalty.cmp(&other.total_penalty);
        }
        // Sort by last AC time
        match (self.last_ac_time, other.last_ac_time) {
            (Some(self_time), Some(other_time)) => self_time.cmp(&other_time),
            (None, None) => self.team_id.cmp(&other.team_id),
            (_, _) => {
                error!(
                    "Cmp branch should not happen, self {:#?} other {:#?}",
                    self, other
                );
                unreachable!()
            }
        }
    }
}
