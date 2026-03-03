mod db;
mod models;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use clap::Parser;
use models::*;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Parser)]
#[command(name = "burrow", about = "Wezel API server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "3001")]
    port: u16,
}

type ApiResult<T> = Result<T, StatusCode>;

fn ise<E: std::fmt::Debug>(_: E) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn scenario_to_json(
    pool: &PgPool,
    id: i64,
    include_graph: bool,
) -> ApiResult<Option<ScenarioJson>> {
    let Some(s) = sqlx::query_as::<_, Scenario>(
        "SELECT id, name, profile, pinned, platform FROM scenarios WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(ise)?
    else {
        return Ok(None);
    };

    let runs = sqlx::query_as::<_, Run>(
        "SELECT \"user\", platform, timestamp, commit_short, build_time_ms, dirty_crates_json \
         FROM runs WHERE scenario_id = $1 ORDER BY timestamp",
    )
    .bind(s.id)
    .fetch_all(pool)
    .await
    .map_err(ise)?;

    let graph = if include_graph {
        sqlx::query_as::<_, GraphRow>(
            "SELECT graph_json FROM scenario_graphs WHERE scenario_id = $1",
        )
        .bind(s.id)
        .fetch_optional(pool)
        .await
        .map_err(ise)?
        .and_then(|r| serde_json::from_str(&r.graph_json).ok())
    } else {
        None
    };

    Ok(Some(ScenarioJson {
        id: s.id,
        name: s.name,
        profile: s.profile,
        pinned: s.pinned,
        platform: s.platform,
        runs: runs.into_iter().map(Run::to_json).collect(),
        graph,
    }))
}

async fn commit_to_json(pool: &PgPool, commit_id: i64) -> ApiResult<CommitJson> {
    let c = sqlx::query_as::<_, Commit>(
        "SELECT sha, short_sha, author, message, timestamp, status FROM commits WHERE id = $1",
    )
    .bind(commit_id)
    .fetch_one(pool)
    .await
    .map_err(ise)?;

    let measurements = sqlx::query_as::<_, Measurement>(
        "SELECT id, name, kind, status, value, prev_value, unit, detail_json \
         FROM measurements WHERE commit_id = $1 ORDER BY id",
    )
    .bind(commit_id)
    .fetch_all(pool)
    .await
    .map_err(ise)?;

    Ok(CommitJson {
        sha: c.sha,
        short_sha: c.short_sha,
        author: c.author,
        message: c.message,
        timestamp: c.timestamp,
        status: c.status,
        measurements: measurements.into_iter().map(Measurement::to_json).collect(),
    })
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn create_project(
    State(pool): State<PgPool>,
    Json(body): Json<Value>,
) -> ApiResult<(StatusCode, Json<Project>)> {
    let name = body["name"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let upstream = body["upstream"].as_str().ok_or(StatusCode::BAD_REQUEST)?;

    let project = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (name, upstream) VALUES ($1, $2) RETURNING id, name, upstream",
    )
    .bind(name)
    .bind(upstream)
    .fetch_one(&pool)
    .await
    .map_err(ise)?;

    Ok((StatusCode::CREATED, Json(project)))
}

async fn get_projects(State(pool): State<PgPool>) -> ApiResult<Json<Vec<Project>>> {
    let projects =
        sqlx::query_as::<_, Project>("SELECT id, name, upstream FROM projects ORDER BY id")
            .fetch_all(&pool)
            .await
            .map_err(ise)?;
    Ok(Json(projects))
}

async fn get_overview(State(pool): State<PgPool>) -> ApiResult<Json<OverviewJson>> {
    let (scenario_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scenarios")
        .fetch_one(&pool)
        .await
        .map_err(ise)?;

    let (tracked_count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM scenarios WHERE pinned = TRUE")
            .fetch_one(&pool)
            .await
            .map_err(ise)?;

    let latest = sqlx::query_as::<_, LatestCommit>(
        "SELECT short_sha, status FROM commits ORDER BY timestamp DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .map_err(ise)?;

    Ok(Json(OverviewJson {
        scenario_count,
        tracked_count,
        latest_commit_short_sha: latest.as_ref().map(|l| l.short_sha.clone()),
        latest_commit_status: latest.map(|l| l.status),
    }))
}

async fn get_scenarios(State(pool): State<PgPool>) -> ApiResult<Json<Vec<ScenarioJson>>> {
    let ids: Vec<(i64,)> = sqlx::query_as("SELECT id FROM scenarios ORDER BY id")
        .fetch_all(&pool)
        .await
        .map_err(ise)?;

    let mut out = Vec::with_capacity(ids.len());
    for (id,) in ids {
        if let Some(s) = scenario_to_json(&pool, id, false).await? {
            out.push(s);
        }
    }
    Ok(Json(out))
}

async fn get_scenario(
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<ScenarioJson>> {
    scenario_to_json(&pool, id, true)
        .await?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn get_scenario_p(
    Path((_pid, id)): Path<(i64, i64)>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<ScenarioJson>> {
    scenario_to_json(&pool, id, true)
        .await?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn get_commits(State(pool): State<PgPool>) -> ApiResult<Json<Vec<CommitJson>>> {
    let ids: Vec<(i64,)> = sqlx::query_as("SELECT id FROM commits ORDER BY timestamp")
        .fetch_all(&pool)
        .await
        .map_err(ise)?;

    let mut out = Vec::with_capacity(ids.len());
    for (id,) in ids {
        out.push(commit_to_json(&pool, id).await?);
    }
    Ok(Json(out))
}

async fn get_commit(
    Path(sha): Path<String>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<CommitJson>> {
    get_commit_inner(&sha, &pool).await
}

async fn get_commit_p(
    Path((_pid, sha)): Path<(i64, String)>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<CommitJson>> {
    get_commit_inner(&sha, &pool).await
}

async fn get_commit_inner(sha: &str, pool: &PgPool) -> ApiResult<Json<CommitJson>> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM commits WHERE sha = $1 OR short_sha = $2")
            .bind(sha)
            .bind(sha)
            .fetch_optional(pool)
            .await
            .map_err(ise)?;

    match row {
        Some((id,)) => Ok(Json(commit_to_json(pool, id).await?)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_users(State(pool): State<PgPool>) -> ApiResult<Json<Vec<String>>> {
    let rows: Vec<User> = sqlx::query_as("SELECT username FROM users ORDER BY username")
        .fetch_all(&pool)
        .await
        .map_err(ise)?;
    Ok(Json(rows.into_iter().map(|u| u.username).collect()))
}

async fn toggle_pin(
    Path(id): Path<i64>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<ScenarioJson>> {
    toggle_pin_inner(id, &pool).await
}

async fn toggle_pin_p(
    Path((_pid, id)): Path<(i64, i64)>,
    State(pool): State<PgPool>,
) -> ApiResult<Json<ScenarioJson>> {
    toggle_pin_inner(id, &pool).await
}

async fn toggle_pin_inner(id: i64, pool: &PgPool) -> ApiResult<Json<ScenarioJson>> {
    let result = sqlx::query("UPDATE scenarios SET pinned = NOT pinned WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(ise)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    scenario_to_json(pool, id, true)
        .await?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn ingest_events(
    State(pool): State<PgPool>,
    Json(events): Json<Vec<Value>>,
) -> ApiResult<StatusCode> {
    for event in &events {
        let Some(upstream) = event.get("upstream").and_then(|v| v.as_str()) else {
            continue;
        };
        let user = event
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let timestamp = event
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let duration_ms = event
            .get("durationMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let run_platform = event.get("platform").and_then(|v| v.as_str()).unwrap_or("");

        // Find or create project
        let name = upstream.rsplit('/').next().unwrap_or(upstream);
        let project_id: i64 =
            match sqlx::query_as::<_, (i64,)>("SELECT id FROM projects WHERE upstream = $1")
                .bind(upstream)
                .fetch_optional(&pool)
                .await
                .map_err(ise)?
            {
                Some((id,)) => id,
                None => {
                    let row = sqlx::query_as::<_, IdRow>(
                        "INSERT INTO projects (name, upstream) VALUES ($1, $2) RETURNING id",
                    )
                    .bind(name)
                    .bind(upstream)
                    .fetch_one(&pool)
                    .await
                    .map_err(ise)?;
                    row.id
                }
            };

        // Ensure user exists
        sqlx::query("INSERT INTO users (username) VALUES ($1) ON CONFLICT (username) DO NOTHING")
            .bind(user)
            .execute(&pool)
            .await
            .map_err(ise)?;

        // Process pheromone data if present
        let Some(pheromone) = event.get("pheromone") else {
            continue;
        };
        let tool = pheromone
            .get("tool")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let command = pheromone
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("build");
        let profile = pheromone
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("dev");
        let packages: Vec<&str> = pheromone
            .get("packages")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        let scenario_platform: Option<&str> = pheromone.get("platform").and_then(|v| v.as_str());

        let scenario_name = if packages.is_empty() {
            format!("{tool} {command}")
        } else {
            format!("{tool} {command} {}", packages.join(" "))
        };

        // Find or create scenario
        let scenario_id: i64 = match if let Some(sp) = scenario_platform {
            sqlx::query_as::<_, (i64,)>(
                "SELECT id FROM scenarios \
                 WHERE project_id = $1 AND name = $2 AND profile = $3 AND platform = $4",
            )
            .bind(project_id)
            .bind(&scenario_name)
            .bind(profile)
            .bind(sp)
            .fetch_optional(&pool)
            .await
        } else {
            sqlx::query_as::<_, (i64,)>(
                "SELECT id FROM scenarios \
                 WHERE project_id = $1 AND name = $2 AND profile = $3 AND platform IS NULL",
            )
            .bind(project_id)
            .bind(&scenario_name)
            .bind(profile)
            .fetch_optional(&pool)
            .await
        }
        .map_err(ise)?
        {
            Some((id,)) => id,
            None => {
                let row = sqlx::query_as::<_, IdRow>(
                    "INSERT INTO scenarios (project_id, name, profile, platform) \
                     VALUES ($1, $2, $3, $4) RETURNING id",
                )
                .bind(project_id)
                .bind(&scenario_name)
                .bind(profile)
                .bind(scenario_platform)
                .fetch_one(&pool)
                .await
                .map_err(ise)?;
                row.id
            }
        };

        // Upsert dependency graph
        if let Some(graph) = pheromone.get("graph") {
            let graph_json = serde_json::to_string(graph).unwrap_or_default();
            sqlx::query(
                "INSERT INTO scenario_graphs (scenario_id, graph_json) VALUES ($1, $2) \
                 ON CONFLICT (scenario_id) DO UPDATE SET graph_json = EXCLUDED.graph_json",
            )
            .bind(scenario_id)
            .bind(&graph_json)
            .execute(&pool)
            .await
            .map_err(ise)?;
        }

        // Insert run
        let commit_short = event.get("commit").and_then(|v| v.as_str()).unwrap_or("");
        let dirty_crates = pheromone.get("dirtyCrates").cloned().unwrap_or(json!([]));
        let dirty_json = serde_json::to_string(&dirty_crates).unwrap_or_else(|_| "[]".into());

        sqlx::query(
            "INSERT INTO runs (scenario_id, \"user\", platform, timestamp, commit_short, build_time_ms, dirty_crates_json) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(scenario_id)
        .bind(user)
        .bind(run_platform)
        .bind(timestamp)
        .bind(commit_short)
        .bind(duration_ms as i64)
        .bind(&dirty_json)
        .execute(&pool)
        .await
        .map_err(ise)?;
    }

    Ok(StatusCode::OK)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let db_url = db::db_url();
    println!("Connecting to database at {db_url}");
    let pool = db::connect(&db_url)
        .await
        .expect("could not connect to database");

    let app = Router::new()
        .route("/api/projects", get(get_projects).post(create_project))
        .route("/api/projects/{project_id}/overview", get(get_overview))
        .route("/api/projects/{project_id}/scenarios", get(get_scenarios))
        .route(
            "/api/projects/{project_id}/scenarios/{id}",
            get(get_scenario_p),
        )
        .route(
            "/api/projects/{project_id}/scenarios/{id}/pin",
            patch(toggle_pin_p),
        )
        .route("/api/projects/{project_id}/commits", get(get_commits))
        .route(
            "/api/projects/{project_id}/commits/{sha}",
            get(get_commit_p),
        )
        .route("/api/projects/{project_id}/users", get(get_users))
        .route("/api/overview", get(get_overview))
        .route("/api/scenarios", get(get_scenarios))
        .route("/api/scenarios/{id}", get(get_scenario))
        .route("/api/scenarios/{id}/pin", patch(toggle_pin))
        .route("/api/commits", get(get_commits))
        .route("/api/commits/{sha}", get(get_commit))
        .route("/api/users", get(get_users))
        .route("/api/events", post(ingest_events))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let addr = format!("0.0.0.0:{}", cli.port);
    println!("Burrow listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
