use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use urlencoding::{decode, encode};

use icloud_calendar_private_api::ICloudCalendarClient;

#[derive(Debug, Deserialize)]
struct Config {
    icloud: ICloudConfig,
    server: ServerConfig,
}

#[derive(Debug, Deserialize)]
struct ICloudConfig {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8888
}

#[derive(Clone)]
struct AppState {
    client: Arc<Mutex<ICloudCalendarClient>>,
}

// Custom error type
enum AppError {
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        AppError::Internal(err.into())
    }
}

#[derive(Serialize)]
struct CalendarEntry {
    display_name: String,
    icloud_url: String,
    api_url: String,
}

#[derive(Serialize)]
struct CalendarListResponse {
    calendars: Vec<CalendarEntry>,
    count: usize,
}

// Handler for GET /list
async fn list_calendars(State(state): State<AppState>) -> Result<Json<CalendarListResponse>, AppError> {
    tracing::info!("Listing calendars");
    
    let mut client = state.client.lock().await;
    let calendars = client.list_calendars().await?;
    
    // Transform CalendarInfo to CalendarEntry with API URLs
    let calendar_entries: Vec<CalendarEntry> = calendars
        .into_iter()
        .map(|cal| {
            let api_url = format!("/calendar/{}", encode(&cal.display_name));
            CalendarEntry {
                display_name: cal.display_name,
                icloud_url: cal.url,
                api_url,
            }
        })
        .collect();
    
    let count = calendar_entries.len();
    tracing::info!("Found {} calendar(s)", count);
    
    Ok(Json(CalendarListResponse { calendars: calendar_entries, count }))
}

// Handler for GET /calendar/:name
async fn get_calendar(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, AppError> {
    // Decode URL-encoded calendar name (handles spaces and special characters)
    let decoded_name = decode(&name)
        .map_err(|e| anyhow::anyhow!("Failed to decode calendar name: {}", e))?;
    
    tracing::info!("Fetching calendar: {}", decoded_name);
    
    let mut client = state.client.lock().await;
    let ical_data = client.get_calendar_ical(&decoded_name).await?;
    
    tracing::info!("Successfully fetched calendar: {}", decoded_name);
    
    // Return as plain text/calendar content type (inline, not as attachment)
    Ok((
        StatusCode::OK,
        [("Content-Type", "text/calendar; charset=utf-8")],
        ical_data,
    )
        .into_response())
}

// Handler for GET /
async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "service": "iCloud Calendar Export API",
        "version": "0.1.0",
        "endpoints": {
            "/": "This help message",
            "/list": "List all available calendars",
            "/calendar/:name": "Get calendar by name (returns iCal format)"
        }
    }))
}

// Handler for GET /health
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "icloud_calendar_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    // Try system-wide config first, then fall back to local directory
    let config_paths = [
        "/etc/icloudcalendarapi/config.toml",
        "config.toml",
        "./config.toml",
    ];
    
    let mut config_content = None;
    let mut config_path_used = None;
    
    for path in &config_paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            config_content = Some(content);
            config_path_used = Some(path);
            break;
        }
    }
    
    let config_content = config_content.expect(
        "Failed to read config.toml.\n\nTried:\n  - /etc/icloudcalendarapi/config.toml\n  - config.toml\n  - ./config.toml\n\nPlease create one from config.example.toml"
    );
    
    let config: Config = toml::from_str(&config_content)
        .expect("Failed to parse config.toml");

    tracing::info!("Loaded configuration from: {}", config_path_used.unwrap());
    tracing::info!("iCloud username: {}", config.icloud.username);

    // Create iCloud client
    let client = ICloudCalendarClient::new(
        config.icloud.username.clone(),
        config.icloud.password.clone(),
    )?;

    let state = AppState {
        client: Arc::new(Mutex::new(client)),
    };

    // Build router
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/list", get(list_calendars))
        .route("/calendar/:name", get(get_calendar))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("🚀 Server listening on http://{}", addr);
    tracing::info!("📋 API Documentation:");
    tracing::info!("  GET  /              - API information");
    tracing::info!("  GET  /health        - Health check");
    tracing::info!("  GET  /list          - List all calendars");
    tracing::info!("  GET  /calendar/:name - Get calendar by name");

    axum::serve(listener, app).await?;

    Ok(())
}
