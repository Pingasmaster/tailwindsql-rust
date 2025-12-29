#![allow(clippy::multiple_crate_versions)]

use std::collections::BTreeMap;
use std::fmt::Write;
use std::sync::{Arc, Mutex};

use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tower_http::services::ServeDir;
use tracing::{error, info};

use tailwindsql::db::{self, DbError};
use tailwindsql::parser::{
    config_with_join, join_config_from_parts, parse_class_names, parse_join_param, JoinConfig,
    QueryConfig,
};
use tailwindsql::query_builder::{build_query, BuiltQuery, QueryBuilderError};
use tailwindsql::render::{render_results, RenderAs, RowData};

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<rusqlite::Connection>>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("db error: {0}")]
    Db(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("query error: {0}")]
    Query(#[from] QueryBuilderError),
    #[error("task join error")]
    Join,
    #[error("db lock error")]
    Lock,
    #[error("invalid query configuration")]
    InvalidConfig,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        error!("{}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    hero_value: String,
    examples: Vec<ExampleCard>,
}

#[derive(Clone)]
struct ExampleCard {
    title: String,
    description: String,
    code_html: String,
    output_html: String,
}

#[derive(Template)]
#[template(path = "explorer.html")]
struct ExplorerTemplate;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_init = db::init_db()?;
    info!("Database path: {:?}", db_init.path);
    if db_init.seeded {
        info!("Database seeded on startup");
    }

    let state = AppState {
        db: Arc::new(Mutex::new(db_init.connection)),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/explorer", get(explorer_handler))
        .route("/api/query", get(query_api_handler))
        .route("/api/schema", get(schema_api_handler))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn index_handler(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let hero_value = with_db(state.clone(), |conn| {
        let config = parse_class_names("db-users-name-where-id-1").ok_or(AppError::InvalidConfig)?;
        let output = execute_query(conn, &config)?;
        let html = render_results(&output.rows, &output.display_columns, RenderAs::Span);
        Ok(strip_tags(&html))
    })
    .await?;

    let examples = with_db(state, build_examples).await?;

    let template = IndexTemplate {
        hero_value,
        examples,
    };
    Ok(Html(template.render().unwrap()))
}

async fn explorer_handler() -> Html<String> {
    let template = ExplorerTemplate;
    Html(template.render().unwrap())
}

#[derive(Deserialize)]
struct QueryParams {
    #[serde(rename = "className")]
    class_name: Option<String>,
    join: Option<String>,
}

#[derive(Serialize)]
struct QueryResponse {
    success: bool,
    query: String,
    params: Vec<JsonValue>,
    results: Vec<RowData>,
    count: usize,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn query_api_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> axum::response::Response {
    let Some(class_name) = params.class_name else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing className parameter".to_string(),
            }),
        )
            .into_response();
    };

    let Some(config) = parse_class_names(&class_name) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid TailwindSQL class: {class_name}"),
            }),
        )
            .into_response();
    };

    let config = if let Some(join_param) = params.join {
        if let Some(join) = parse_join_param(&join_param) {
            config_with_join(config, join)
        } else {
            config
        }
    } else {
        config
    };

    let result = with_db(state, move |conn| execute_query(conn, &config)).await;
    match result {
        Ok(result) => {
            let count = result.rows.len();
            (
                StatusCode::OK,
                Json(QueryResponse {
                    success: true,
                    query: result.sql,
                    params: result.params,
                    results: result.rows,
                    count,
                }),
            )
                .into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
struct SchemaResponse {
    tables: Vec<TableInfo>,
}

#[derive(Serialize)]
struct TableInfo {
    name: String,
    columns: Vec<ColumnInfo>,
    #[serde(rename = "rowCount")]
    row_count: i64,
    data: Vec<RowData>,
}

#[derive(Serialize)]
struct ColumnInfo {
    name: String,
    #[serde(rename = "type")]
    col_type: String,
}

async fn schema_api_handler(State(state): State<AppState>) -> impl IntoResponse {
    let result = with_db(state, |conn| {
        let mut tables = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )?;
        let table_names = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        for table_name in table_names {
            let mut pragma = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
            let columns = pragma
                .query_map([], |row| {
                    let name: String = row.get(1)?;
                    let col_type: String = row.get(2)?;
                    Ok(ColumnInfo {
                        name,
                        col_type: if col_type.is_empty() { "TEXT".to_string() } else { col_type },
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            let row_count: i64 = conn.query_row(
                &format!("SELECT COUNT(*) FROM {table_name}"),
                [],
                |row| row.get(0),
            )?;

            let data = fetch_table_rows(conn, &table_name, 20)?;

            tables.push(TableInfo {
                name: table_name,
                columns,
                row_count,
                data,
            });
        }

        Ok::<_, AppError>(SchemaResponse { tables })
    })
    .await;

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

struct QueryOutput {
    sql: String,
    params: Vec<JsonValue>,
    rows: Vec<RowData>,
    display_columns: Vec<String>,
}

async fn with_db<T, F>(state: AppState, func: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&rusqlite::Connection) -> Result<T, AppError> + Send + 'static,
{
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let guard = db.lock().map_err(|_| AppError::Lock)?;
        func(&guard)
    })
    .await
    .map_err(|_| AppError::Join)?
}

fn execute_query(conn: &rusqlite::Connection, config: &QueryConfig) -> Result<QueryOutput, AppError> {
    let built = build_query(config)?;
    let BuiltQuery { sql, params } = built;
    let (rows, columns) = run_query(conn, &sql, &params)?;

    let mut display_columns = config.columns.clone();
    for join in &config.joins {
        display_columns.extend(join.columns.iter().cloned());
    }
    if display_columns.is_empty() {
        display_columns = columns;
    }

    Ok(QueryOutput {
        sql,
        params: params.iter().cloned().map(sqlite_value_to_json).collect(),
        rows,
        display_columns,
    })
}

fn run_query(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> Result<(Vec<RowData>, Vec<String>), AppError> {
    let mut stmt = conn.prepare(sql)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(ToString::to_string).collect();
    let names = column_names.clone();
    let rows_iter = stmt.query_map(rusqlite::params_from_iter(params.iter()), {
        move |row| {
            let mut data = BTreeMap::new();
            for (i, name) in names.iter().enumerate() {
                let value: rusqlite::types::Value = row.get(i)?;
                data.insert(name.clone(), sqlite_value_to_json(value));
            }
            Ok(data)
        }
    })?;

    let mut rows = Vec::new();
    for row in rows_iter {
        rows.push(row?);
    }

    Ok((rows, column_names))
}

fn fetch_table_rows(
    conn: &rusqlite::Connection,
    table: &str,
    limit: usize,
) -> Result<Vec<RowData>, AppError> {
    let sql = format!("SELECT * FROM {table} LIMIT {limit}");
    let (rows, _) = run_query(conn, &sql, &[])?;
    Ok(rows)
}

fn sqlite_value_to_json(value: rusqlite::types::Value) -> JsonValue {
    match value {
        rusqlite::types::Value::Null => JsonValue::Null,
        rusqlite::types::Value::Integer(v) => JsonValue::Number(v.into()),
        rusqlite::types::Value::Real(v) => serde_json::Number::from_f64(v)
            .map_or(JsonValue::Null, JsonValue::Number),
        rusqlite::types::Value::Text(v) => JsonValue::String(v),
        rusqlite::types::Value::Blob(bytes) => {
            let mut hex = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                write!(&mut hex, "{byte:02x}").expect("writing to String should not fail");
            }
            JsonValue::String(format!("0x{hex}"))
        }
    }
}

fn strip_tags(input: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn build_examples(conn: &rusqlite::Connection) -> Result<Vec<ExampleCard>, AppError> {
    let mut examples = Vec::new();

    examples.push(build_example_card(
        conn,
        "Get User Name",
        "Fetch a single user's name by ID",
        "db-users-name-where-id-1",
        RenderAs::Span,
        None,
        None,
    )?);

    examples.push(build_example_card(
        conn,
        "Product List",
        "Display products as an unordered list",
        "db-products-title-limit-5",
        RenderAs::Ul,
        None,
        None,
    )?);

    examples.push(build_example_card(
        conn,
        "Top Posts by Likes",
        "Posts ordered by popularity",
        "db-posts-title-orderby-likes-desc-limit-3",
        RenderAs::Ol,
        None,
        None,
    )?);

    let join = join_config_from_parts("posts", "id-author_id", Some("title"), Some("left"));
    examples.push(build_example_card(
        conn,
        "Users with Posts (JOIN)",
        "Join users with their posts",
        "db-users-name-limit-5",
        RenderAs::Table,
        Some(join),
        Some(join_code_preview()),
    )?);

    Ok(examples)
}

fn build_example_card(
    conn: &rusqlite::Connection,
    title: &str,
    description: &str,
    class_name: &str,
    render_as: RenderAs,
    join: Option<JoinConfig>,
    code_override: Option<String>,
) -> Result<ExampleCard, AppError> {
    let mut config = parse_class_names(class_name).ok_or(AppError::InvalidConfig)?;
    if let Some(join) = join {
        config.joins.push(join);
    }
    let output = execute_query(conn, &config)?;

    let output_html = render_results(&output.rows, &output.display_columns, render_as);
    let code_html = code_override.unwrap_or_else(|| {
        let as_fragment = if matches!(render_as, RenderAs::Span) {
            String::new()
        } else {
            let label = render_as_label(render_as);
            format!(
                " <span><span class=\"text-slate-300\">as=</span><span class=\"text-green-400\">\"{label}\"</span></span>"
            )
        };

        format!(
            "<div class=\"flex flex-wrap items-baseline gap-x-1\"><span class=\"text-pink-400\">&lt;DB</span><span><span class=\"text-slate-300\">className=</span><span class=\"text-green-400\">\"{class_name}\"</span></span>{as_fragment}<span class=\"text-pink-400\">/&gt;</span></div>"
        )
    });

    Ok(ExampleCard {
        title: title.to_string(),
        description: description.to_string(),
        code_html,
        output_html,
    })
}

const fn render_as_label(render_as: RenderAs) -> &'static str {
    match render_as {
        RenderAs::Ul => "ul",
        RenderAs::Ol => "ol",
        RenderAs::Table => "table",
        RenderAs::Json => "json",
        RenderAs::Code => "code",
        RenderAs::Div => "div",
        RenderAs::Span => "span",
    }
}

fn join_code_preview() -> String {
    let mut html = String::new();
    html.push_str(
        "<div class=\"flex flex-col\">\
        <div class=\"flex flex-wrap items-baseline gap-x-1\">\
        <span class=\"text-pink-400\">&lt;DB</span>\
        <span><span class=\"text-slate-300\">className=</span><span class=\"text-green-400\">\"db-users-name-limit-5\"</span></span>\
        <span><span class=\"text-slate-300\">as=</span><span class=\"text-green-400\">\"table\"</span></span>\
        <span class=\"text-pink-400\">&gt;</span>\
        </div>\
        <div class=\"flex flex-wrap items-baseline gap-x-1 pl-4\">\
        <span class=\"text-purple-400\">&lt;Join</span>\
        <span><span class=\"text-slate-300\">table=</span><span class=\"text-green-400\">\"posts\"</span></span>\
        <span><span class=\"text-slate-300\">on=</span><span class=\"text-yellow-400\">\"id-author_id\"</span></span>\
        <span><span class=\"text-slate-300\">select=</span><span class=\"text-green-400\">\"title\"</span></span>\
        <span class=\"text-purple-400\">/&gt;</span>\
        </div>\
        <span class=\"text-pink-400\">&lt;/DB&gt;</span>\
        </div>");
    html
}
