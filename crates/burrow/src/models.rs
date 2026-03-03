use serde::Serialize;
use sqlx::FromRow;

// ── DB rows ──────────────────────────────────────────────────────────────────

#[derive(FromRow, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub upstream: String,
}

#[derive(FromRow, Serialize)]
pub struct User {
    pub username: String,
}

#[derive(FromRow, Serialize)]
pub struct Scenario {
    pub id: i64,
    pub name: String,
    pub profile: String,
    pub pinned: bool,
    pub platform: Option<String>,
}

#[derive(FromRow)]
pub struct Run {
    pub user: String,
    pub platform: String,
    pub timestamp: String,
    pub commit_short: String,
    pub build_time_ms: i64,
    pub dirty_crates_json: String,
}

#[derive(FromRow)]
pub struct Commit {
    pub sha: String,
    pub short_sha: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
    pub status: String,
}

#[derive(FromRow)]
pub struct Measurement {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub status: String,
    pub value: Option<f64>,
    pub prev_value: Option<f64>,
    pub unit: Option<String>,
    pub detail_json: Option<String>,
}

#[derive(FromRow)]
pub struct IdRow {
    pub id: i64,
}

#[derive(FromRow)]
pub struct GraphRow {
    pub graph_json: String,
}

#[derive(FromRow)]
pub struct LatestCommit {
    pub short_sha: String,
    pub status: String,
}

// ── API responses ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct RunJson {
    pub user: String,
    pub platform: String,
    pub timestamp: String,
    pub commit: String,
    #[serde(rename = "buildTimeMs")]
    pub build_time_ms: i64,
    #[serde(rename = "dirtyCrates")]
    pub dirty_crates: serde_json::Value,
}

#[derive(Serialize)]
pub struct ScenarioJson {
    pub id: i64,
    pub name: String,
    pub profile: String,
    pub pinned: bool,
    pub platform: Option<String>,
    pub runs: Vec<RunJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct MeasurementJson {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "prevValue")]
    pub prev_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct CommitJson {
    pub sha: String,
    #[serde(rename = "shortSha")]
    pub short_sha: String,
    pub author: String,
    pub message: String,
    pub timestamp: String,
    pub status: String,
    pub measurements: Vec<MeasurementJson>,
}

#[derive(Serialize)]
pub struct OverviewJson {
    #[serde(rename = "scenarioCount")]
    pub scenario_count: i64,
    #[serde(rename = "trackedCount")]
    pub tracked_count: i64,
    #[serde(rename = "latestCommitShortSha")]
    pub latest_commit_short_sha: Option<String>,
    #[serde(rename = "latestCommitStatus")]
    pub latest_commit_status: Option<String>,
}

// ── Conversions ──────────────────────────────────────────────────────────────

impl Run {
    pub fn to_json(self) -> RunJson {
        let dirty_crates: serde_json::Value =
            serde_json::from_str(&self.dirty_crates_json).unwrap_or(serde_json::json!([]));
        RunJson {
            user: self.user,
            platform: self.platform,
            timestamp: self.timestamp,
            commit: self.commit_short,
            build_time_ms: self.build_time_ms,
            dirty_crates,
        }
    }
}

impl Measurement {
    pub fn to_json(self) -> MeasurementJson {
        let detail = self.detail_json.and_then(|d| serde_json::from_str(&d).ok());
        MeasurementJson {
            id: self.id,
            name: self.name,
            kind: self.kind,
            status: self.status,
            value: self.value,
            prev_value: self.prev_value,
            unit: self.unit,
            detail,
        }
    }
}
