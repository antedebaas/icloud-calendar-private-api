use axum::{
    extract::{Path, State, Request},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
    middleware::{self, Next},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use urlencoding::{decode, encode};
use base64::{Engine as _, engine::general_purpose};

use icloud_calendar_private_api::ICloudCalendarClient;

#[derive(Debug, Deserialize)]
struct Config {
    icloud: ICloudConfig,
    server: ServerConfig,
    #[serde(default)]
    stalwart: StalwartConfig,
}

#[derive(Debug, Deserialize)]
struct ICloudConfig {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    public_url: Option<String>,
    #[serde(default)]
    public_path: Option<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8888
}

#[derive(Debug, Deserialize, Clone)]
struct StalwartConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_stalwart_url")]
    server_url: String,
    #[serde(default = "default_auth_method")]
    auth_method: String,
}

impl Default for StalwartConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_url: default_stalwart_url(),
            auth_method: default_auth_method(),
        }
    }
}

fn default_stalwart_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_auth_method() -> String {
    "jmap".to_string()
}

#[derive(Clone)]
struct AppState {
    client: Arc<Mutex<ICloudCalendarClient>>,
    stalwart_config: StalwartConfig,
    server_config: ServerConfig,
}

// Stalwart authentication validator
async fn validate_stalwart_credentials(
    config: &StalwartConfig,
    username: &str,
    password: &str,
) -> Result<bool, anyhow::Error> {
    if !config.enabled {
        return Ok(true); // Authentication disabled, allow access
    }

    let client = reqwest::Client::new();
    
    match config.auth_method.as_str() {
        "jmap" => {
            // JMAP authentication endpoint
            let url = format!("{}/.well-known/jmap", config.server_url);
            let response = client
                .get(&url)
                .basic_auth(username, Some(password))
                .send()
                .await?;
            
            Ok(response.status().is_success())
        }
        "imap" => {
            // For IMAP, we can try to authenticate via a simple IMAP login
            // This is a simplified check - in production, you might want to use a proper IMAP client
            // For now, we'll use JMAP as the default and log a warning
            tracing::warn!("IMAP authentication not fully implemented, falling back to JMAP");
            let url = format!("{}/.well-known/jmap", config.server_url);
            let response = client
                .get(&url)
                .basic_auth(username, Some(password))
                .send()
                .await?;
            
            Ok(response.status().is_success())
        }
        _ => {
            tracing::error!("Unknown authentication method: {}", config.auth_method);
            Ok(false)
        }
    }
}

// Authentication middleware
async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // If authentication is disabled, proceed without checks
    if !state.stalwart_config.enabled {
        return Ok(next.run(request).await);
    }

    // Extract Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_value) = auth_header {
        if auth_value.starts_with("Basic ") {
            // Decode Basic Auth credentials
            let encoded = &auth_value[6..];
            if let Ok(decoded_bytes) = general_purpose::STANDARD.decode(encoded) {
                if let Ok(decoded) = String::from_utf8(decoded_bytes) {
                    let parts: Vec<&str> = decoded.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        let username = parts[0];
                        let password = parts[1];

                        // Validate credentials against Stalwart
                        match validate_stalwart_credentials(&state.stalwart_config, username, password).await {
                            Ok(true) => {
                                tracing::debug!("Authentication successful for user: {}", username);
                                return Ok(next.run(request).await);
                            }
                            Ok(false) => {
                                tracing::warn!("Authentication failed for user: {}", username);
                            }
                            Err(e) => {
                                tracing::error!("Authentication error: {}", e);
                                return Err(AppError::Internal(anyhow::anyhow!("Authentication service error")));
                            }
                        }
                    }
                }
            }
        }
    }

    // Authentication failed or missing
    Ok((
        StatusCode::UNAUTHORIZED,
        [("WWW-Authenticate", "Basic realm=\"iCloud Calendar API\"")],
        Json(serde_json::json!({
            "error": "Authentication required"
        })),
    )
        .into_response())
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
    
    // Construct base URL from server config
    // Use public_url if set, otherwise use host:port
    let base_url = if let Some(public_url) = &state.server_config.public_url {
        public_url.trim_end_matches('/').to_string()
    } else {
        format!("http://{}:{}", state.server_config.host, state.server_config.port)
    };
    
    // Add public_path if configured
    let base_path = if let Some(public_path) = &state.server_config.public_path {
        format!("{}/{}", base_url, public_path.trim_matches('/'))
    } else {
        base_url
    };
    
    // Transform CalendarInfo to CalendarEntry with full API URLs
    let calendar_entries: Vec<CalendarEntry> = calendars
        .into_iter()
        .map(|cal| {
            let api_url = format!("{}/calendar/{}", base_path, encode(&cal.display_name));
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
        "service": "iCloud Calendar Private API",
        "version": "1.2.0",
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
    
    // Log Stalwart authentication status
    if config.stalwart.enabled {
        tracing::info!("🔒 Stalwart authentication: ENABLED");
        tracing::info!("   Server URL: {}", config.stalwart.server_url);
        tracing::info!("   Auth method: {}", config.stalwart.auth_method);
    } else {
        tracing::warn!("🔓 Stalwart authentication: DISABLED (API endpoints are unprotected)");
    }
    
    // Log public URL configuration
    if config.server.public_url.is_some() || config.server.public_path.is_some() {
        tracing::info!("🌍 Public URL configuration:");
        if let Some(url) = &config.server.public_url {
            tracing::info!("   Public URL: {}", url);
        }
        if let Some(path) = &config.server.public_path {
            tracing::info!("   Public path: /{}", path.trim_matches('/'));
        }
    }

    // Create iCloud client
    let client = ICloudCalendarClient::new(
        config.icloud.username.clone(),
        config.icloud.password.clone(),
    )?;

    let state = AppState {
        client: Arc::new(Mutex::new(client)),
        stalwart_config: config.stalwart.clone(),
        server_config: config.server.clone(),
    };

    // Build router
    // Public endpoints (no authentication required)
    let public_routes = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    // Protected endpoints (require authentication)
    let protected_routes = Router::new()
        .route("/list", get(list_calendars))
        .route("/calendar/:name", get(get_calendar))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Combine routes
    let app = public_routes
        .merge(protected_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

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
