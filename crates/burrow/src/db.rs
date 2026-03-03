use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    let pool = PgPoolOptions::new().max_connections(5).connect(url).await?;

    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::raw_sql(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            upstream TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS scenarios (
            id BIGSERIAL PRIMARY KEY,
            project_id BIGINT NOT NULL REFERENCES projects(id),
            name TEXT NOT NULL,
            profile TEXT NOT NULL CHECK(profile IN ('dev', 'release')),
            pinned BOOLEAN NOT NULL DEFAULT FALSE,
            platform TEXT
        );
        CREATE TABLE IF NOT EXISTS scenario_graphs (
            scenario_id BIGINT PRIMARY KEY REFERENCES scenarios(id),
            graph_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS runs (
            id BIGSERIAL PRIMARY KEY,
            scenario_id BIGINT NOT NULL REFERENCES scenarios(id),
            \"user\" TEXT NOT NULL,
            platform TEXT NOT NULL DEFAULT '',
            timestamp TEXT NOT NULL,
            commit_short TEXT NOT NULL,
            build_time_ms BIGINT NOT NULL,
            dirty_crates_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS commits (
            id BIGSERIAL PRIMARY KEY,
            project_id BIGINT NOT NULL REFERENCES projects(id),
            sha TEXT NOT NULL,
            short_sha TEXT NOT NULL,
            author TEXT NOT NULL,
            message TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            status TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS measurements (
            id BIGSERIAL PRIMARY KEY,
            commit_id BIGINT NOT NULL REFERENCES commits(id),
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            value DOUBLE PRECISION,
            prev_value DOUBLE PRECISION,
            unit TEXT,
            detail_json TEXT
        );
        ",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("BURROW_DB"))
        .unwrap_or_else(|_| "postgres://localhost/burrow".to_string())
}
