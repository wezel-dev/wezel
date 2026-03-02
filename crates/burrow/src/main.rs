mod db;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use clap::Parser;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Parser)]
#[command(name = "burrow", about = "Wezel API server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value = "3001")]
    port: u16,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

async fn scenario_to_json(
    pool: &SqlitePool,
    id: i64,
    include_graph: bool,
) -> Result<Option<Value>, StatusCode> {
    let row = sqlx::query("SELECT id, name, profile, pinned, platform FROM scenarios WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Ok(None);
    };

    let sid: i64 = row.get("id");
    let name: String = row.get("name");
    let profile: String = row.get("profile");
    let pinned: bool = row.get("pinned");
    let platform: Option<String> = row.get("platform");

    let mut obj = json!({
        "id": sid,
        "name": name,
        "profile": profile,
        "pinned": pinned,
        "platform": platform,
    });

    // Attach runs
    let run_rows = sqlx::query(
        "SELECT user, platform, timestamp, commit_short, build_time_ms, dirty_crates_json \
         FROM runs WHERE scenario_id = ? ORDER BY timestamp",
    )
    .bind(sid)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let runs_json: Vec<Value> = run_rows
        .into_iter()
        .map(|r| {
            let dirty_str: String = r.get("dirty_crates_json");
            let dirty: Value = serde_json::from_str(&dirty_str).unwrap_or(json!([]));
            json!({
                "user": r.get::<String, _>("user"),
                "platform": r.get::<String, _>("platform"),
                "timestamp": r.get::<String, _>("timestamp"),
                "commit": r.get::<String, _>("commit_short"),
                "buildTimeMs": r.get::<i64, _>("build_time_ms"),
                "dirtyCrates": dirty,
            })
        })
        .collect();
    obj["runs"] = json!(runs_json);

    if include_graph {
        let graph_row = sqlx::query("SELECT graph_json FROM scenario_graphs WHERE scenario_id = ?")
            .bind(sid)
            .fetch_optional(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let graph: Value = graph_row
            .and_then(|r| {
                let g: String = r.get("graph_json");
                serde_json::from_str(&g).ok()
            })
            .unwrap_or(json!([]));
        obj["graph"] = graph;
    }

    Ok(Some(obj))
}

async fn commit_to_json(pool: &SqlitePool, commit_id: i64) -> Result<Value, StatusCode> {
    let c = sqlx::query(
        "SELECT sha, short_sha, author, message, timestamp, status FROM commits WHERE id = ?",
    )
    .bind(commit_id)
    .fetch_one(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let m_rows = sqlx::query(
        "SELECT id, name, kind, status, value, prev_value, unit, detail_json \
         FROM measurements WHERE commit_id = ? ORDER BY id",
    )
    .bind(commit_id)
    .fetch_all(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let measurements: Vec<Value> = m_rows
        .into_iter()
        .map(|r| {
            let mut m = json!({
                "id": r.get::<i64, _>("id"),
                "name": r.get::<String, _>("name"),
                "kind": r.get::<String, _>("kind"),
                "status": r.get::<String, _>("status"),
            });
            if let Ok(v) = r.try_get::<f64, _>("value") {
                m["value"] = json!(v);
            }
            if let Ok(v) = r.try_get::<f64, _>("prev_value") {
                m["prevValue"] = json!(v);
            }
            if let Ok(v) = r.try_get::<String, _>("unit") {
                m["unit"] = json!(v);
            }
            if let Ok(d) = r.try_get::<String, _>("detail_json")
                && let Ok(parsed) = serde_json::from_str::<Value>(&d)
            {
                m["detail"] = parsed;
            }
            m
        })
        .collect();

    Ok(json!({
        "sha": c.get::<String, _>("sha"),
        "shortSha": c.get::<String, _>("short_sha"),
        "author": c.get::<String, _>("author"),
        "message": c.get::<String, _>("message"),
        "timestamp": c.get::<String, _>("timestamp"),
        "status": c.get::<String, _>("status"),
        "measurements": measurements,
    }))
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn create_project(
    State(pool): State<SqlitePool>,
    Json(body): Json<Value>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let name = body["name"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let upstream = body["upstream"].as_str().ok_or(StatusCode::BAD_REQUEST)?;

    let result = sqlx::query("INSERT INTO projects (name, upstream) VALUES (?, ?)")
        .bind(name)
        .bind(upstream)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id = result.last_insert_rowid();
    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "name": name, "upstream": upstream })),
    ))
}

async fn get_projects(State(pool): State<SqlitePool>) -> Result<Json<Value>, StatusCode> {
    let rows = sqlx::query("SELECT id, name, upstream FROM projects ORDER BY id")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let projects: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"),
                "name": r.get::<String, _>("name"),
                "upstream": r.get::<String, _>("upstream"),
            })
        })
        .collect();
    Ok(Json(json!(projects)))
}

async fn get_overview(State(pool): State<SqlitePool>) -> Result<Json<Value>, StatusCode> {
    let sc: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scenarios")
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tc: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scenarios WHERE pinned = 1")
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let latest =
        sqlx::query("SELECT short_sha, status FROM commits ORDER BY timestamp DESC LIMIT 1")
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "scenarioCount": sc.0,
        "trackedCount": tc.0,
        "latestCommitShortSha": latest.as_ref().map(|r| r.get::<String, _>("short_sha")),
        "latestCommitStatus": latest.as_ref().map(|r| r.get::<String, _>("status")),
    })))
}

async fn get_scenarios(State(pool): State<SqlitePool>) -> Result<Json<Value>, StatusCode> {
    let id_rows: Vec<(i64,)> = sqlx::query_as("SELECT id FROM scenarios ORDER BY id")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut scenarios = Vec::new();
    for (id,) in id_rows {
        if let Some(s) = scenario_to_json(&pool, id, false).await? {
            scenarios.push(s);
        }
    }
    Ok(Json(json!(scenarios)))
}

async fn get_scenario(
    Path(id): Path<i64>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    match scenario_to_json(&pool, id, true).await? {
        Some(s) => Ok(Json(s)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_scenario_p(
    Path((_pid, id)): Path<(i64, i64)>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    match scenario_to_json(&pool, id, true).await? {
        Some(s) => Ok(Json(s)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_commits(State(pool): State<SqlitePool>) -> Result<Json<Value>, StatusCode> {
    let id_rows: Vec<(i64,)> = sqlx::query_as("SELECT id FROM commits ORDER BY timestamp")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut commits = Vec::new();
    for (id,) in id_rows {
        commits.push(commit_to_json(&pool, id).await?);
    }
    Ok(Json(json!(commits)))
}

async fn get_commit(
    Path(sha): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    get_commit_inner(&sha, &pool).await
}

async fn get_commit_p(
    Path((_pid, sha)): Path<(i64, String)>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    get_commit_inner(&sha, &pool).await
}

async fn get_commit_inner(sha: &str, pool: &SqlitePool) -> Result<Json<Value>, StatusCode> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM commits WHERE sha = ? OR short_sha = ?")
            .bind(sha)
            .bind(sha)
            .fetch_optional(pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some((id,)) => Ok(Json(commit_to_json(pool, id).await?)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_users(State(pool): State<SqlitePool>) -> Result<Json<Value>, StatusCode> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT username FROM users ORDER BY username")
        .fetch_all(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users: Vec<String> = rows.into_iter().map(|(u,)| u).collect();
    Ok(Json(json!(users)))
}

async fn toggle_pin(
    Path(id): Path<i64>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    toggle_pin_inner(id, &pool).await
}

async fn toggle_pin_p(
    Path((_pid, id)): Path<(i64, i64)>,
    State(pool): State<SqlitePool>,
) -> Result<Json<Value>, StatusCode> {
    toggle_pin_inner(id, &pool).await
}

async fn ingest_events(
    State(pool): State<SqlitePool>,
    Json(events): Json<Vec<Value>>,
) -> Result<StatusCode, StatusCode> {
    for event in &events {
        let upstream = match event.get("upstream").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => continue,
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
            match sqlx::query_as::<_, (i64,)>("SELECT id FROM projects WHERE upstream = ?")
                .bind(upstream)
                .fetch_optional(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                Some((id,)) => id,
                None => sqlx::query("INSERT INTO projects (name, upstream) VALUES (?, ?)")
                    .bind(name)
                    .bind(upstream)
                    .execute(&pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .last_insert_rowid(),
            };

        // Ensure user exists
        sqlx::query("INSERT OR IGNORE INTO users (username) VALUES (?)")
            .bind(user)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
                "SELECT id FROM scenarios WHERE project_id = ? AND name = ? AND profile = ? AND platform = ?",
            )
            .bind(project_id)
            .bind(&scenario_name)
            .bind(profile)
            .bind(sp)
            .fetch_optional(&pool)
            .await
        } else {
            sqlx::query_as::<_, (i64,)>(
                "SELECT id FROM scenarios WHERE project_id = ? AND name = ? AND profile = ? AND platform IS NULL",
            )
            .bind(project_id)
            .bind(&scenario_name)
            .bind(profile)
            .fetch_optional(&pool)
            .await
        }
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            Some((id,)) => id,
            None => {
                sqlx::query("INSERT INTO scenarios (project_id, name, profile, platform) VALUES (?, ?, ?, ?)")
                    .bind(project_id)
                    .bind(&scenario_name)
                    .bind(profile)
                    .bind(scenario_platform)
                    .execute(&pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .last_insert_rowid()
            }
        };

        // Upsert dependency graph
        if let Some(graph) = pheromone.get("graph") {
            let graph_json = serde_json::to_string(graph).unwrap_or_default();
            sqlx::query(
                "INSERT INTO scenario_graphs (scenario_id, graph_json) VALUES (?, ?) \
                 ON CONFLICT(scenario_id) DO UPDATE SET graph_json = excluded.graph_json",
            )
            .bind(scenario_id)
            .bind(&graph_json)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }

        // Insert run
        let commit_short = event.get("commit").and_then(|v| v.as_str()).unwrap_or("");
        let dirty_crates = pheromone.get("dirtyCrates").cloned().unwrap_or(json!([]));
        let dirty_json = serde_json::to_string(&dirty_crates).unwrap_or_else(|_| "[]".into());

        sqlx::query(
            "INSERT INTO runs (scenario_id, user, platform, timestamp, commit_short, build_time_ms, dirty_crates_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

async fn toggle_pin_inner(id: i64, pool: &SqlitePool) -> Result<Json<Value>, StatusCode> {
    let result = sqlx::query("UPDATE scenarios SET pinned = NOT pinned WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    match scenario_to_json(pool, id, true).await? {
        Some(s) => Ok(Json(s)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let db_url = db::db_url();
    println!("Opening database at {db_url}");
    let pool = db::open(&db_url).await.expect("could not open database");

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
