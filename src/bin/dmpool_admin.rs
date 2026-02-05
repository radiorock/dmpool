// DMPool Admin Server
// Standalone admin web interface for pool management

use anyhow::Result;
use axum::{
    extract::{Path, Query, State, Request},
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
    middleware,
};
use chrono::Utc;
use p2poolv2_lib::config::Config;
use p2poolv2_lib::shares::chain::chain_store::ChainStore;
use p2poolv2_lib::shares::share_block::ShareBlock;
use p2poolv2_lib::store::Store;
use dmpool::auth::{AuthManager, LoginRequest, LoginResponse, UserInfo};
use dmpool::audit::{AuditLogger, AuditFilter};
use dmpool::backup::{BackupManager, BackupConfig, BackupStats};
use dmpool::confirmation::ConfigConfirmation;
use dmpool::health::HealthChecker;
use dmpool::payment::{PaymentManager, PaymentConfig, Payout, PayoutStatus, MinerBalance};
use dmpool::two_factor::{TwoFactorManager, TwoFactorSetup, TwoFactorStatus, TwoFactorEnable, TwoFactorLogin};
use dmpool::rate_limit::{RateLimiterState, RateLimitConfig, rate_limit_middleware, login_rate_limit_middleware};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn, Level};

/// Admin state
#[derive(Clone)]
struct AdminState {
    config_path: String,
    config: Arc<RwLock<Config>>,
    store: Arc<Store>,
    chain_store: Arc<ChainStore>,
    health_checker: Arc<HealthChecker>,
    auth_manager: Arc<AuthManager>,
    two_factor_manager: Arc<TwoFactorManager>,
    rate_limiter: Arc<RateLimiterState>,
    audit_logger: Arc<AuditLogger>,
    config_confirmation: Arc<ConfigConfirmation>,
    backup_manager: Arc<BackupManager>,
    payment_manager: Arc<PaymentManager>,
    start_time: std::time::Instant,
    banned_workers: Arc<RwLock<HashSet<String>>>,
    worker_tags: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

// ===== Response Types =====

#[derive(Serialize)]
struct ApiResponse<T> {
    status: String,
    data: Option<T>,
    message: Option<String>,
    timestamp: u64,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            status: "ok".to_string(),
            data: Some(data),
            message: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            data: None,
            message: Some(msg.into()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

#[derive(Serialize)]
struct DashboardMetrics {
    pool_hashrate_ths: f64,
    active_workers: u64,
    total_shares: u64,
    blocks_found: u64,
    uptime_seconds: u64,
    pplns_window_shares: u64,
    current_difficulty: f64,
}

#[derive(Serialize)]
struct ConfigView {
    stratum_port: u16,
    stratum_hostname: String,
    start_difficulty: u64,
    minimum_difficulty: u64,
    pplns_ttl_days: u64,
    difficulty_multiplier: f64,
    network: String,
    pool_signature: Option<String>,
    ignore_difficulty: bool,
    donation: Option<u16>,
    fee: Option<u16>,
}

#[derive(Serialize)]
struct SafetyReport {
    safe: bool,
    critical_issues: Vec<SafetyIssue>,
    warnings: Vec<SafetyIssue>,
}

#[derive(Serialize)]
struct SafetyIssue {
    severity: String,
    param: String,
    message: String,
    recommendation: String,
}

#[derive(Serialize)]
struct WorkerInfo {
    address: String,
    worker_name: String,
    hashrate_ths: f64,
    shares_count: u64,
    difficulty: u64,
    last_seen: String,
    first_seen: String,
    is_banned: bool,
    tags: Vec<String>,
    status: WorkerStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum WorkerStatus {
    Active,
    Inactive,
    Banned,
}

/// Pagination request
#[derive(Deserialize)]
struct PaginationRequest {
    page: Option<usize>,
    page_size: Option<usize>,
    search: Option<String>,
    status: Option<String>,
    sort_by: Option<String>,
    sort_order: Option<String>,
}

/// Paginated response
#[derive(Serialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    total: usize,
    page: usize,
    page_size: usize,
    total_pages: usize,
}

// ===== Request Types =====

#[derive(Deserialize)]
struct ConfigUpdate {
    start_difficulty: Option<u32>,
    minimum_difficulty: Option<u32>,
    pool_signature: Option<String>,
}

#[derive(Deserialize)]
struct BanRequest {
    reason: Option<String>,
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let port: u16 = std::env::var("ADMIN_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap_or(8080);

    // Get admin credentials from environment
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD")
        .unwrap_or_else(|_| {
            warn!("ADMIN_PASSWORD not set, using default password (INSECURE!)");
            "Admin@2026!Default".to_string() // Meets password requirements
        });

    // Get JWT secret - MUST be set in production
    let is_production = std::env::var("DMP_ENV").unwrap_or_else(|_| "development".to_string()) == "production";
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        if is_production {
            error!("JWT_SECRET environment variable MUST be set in production!");
            error!("Generate a secure secret with: openssl rand -base64 32");
            std::process::exit(1);
        } else {
            // For development, generate a random secret each time
            use rand::Rng;
            let secret: String = rand::thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect();
            warn!("Using generated JWT secret for development. Set JWT_SECRET for persistence!");
            secret
        }
    });

    // Validate JWT secret length
    if jwt_secret.len() < 32 {
        error!("JWT_SECRET must be at least 32 characters long! Current length: {}", jwt_secret.len());
        std::process::exit(1);
    }

    // Load config
    let config = Config::load(&config_path)?;
    let store = Arc::new(Store::new(config.store.path.clone(), true)
        .map_err(|e| anyhow::anyhow!("Failed to open store: {}", e))?);
    let genesis = ShareBlock::build_genesis_for_network(config.stratum.network);
    let chain_store = Arc::new(ChainStore::new(
        store.clone(),
        genesis,
        config.stratum.network,
    ));

    // Initialize auth manager
    let auth_manager = Arc::new(AuthManager::new(jwt_secret));
    auth_manager.init_default_admin(&admin_username, &admin_password).await?;
    info!("Initialized admin user: {}", admin_username);

    // Initialize rate limiter
    let rate_limit_config = RateLimitConfig::default();
    let api_rpm = rate_limit_config.api_rpm.get();
    let login_rpm = rate_limit_config.login_rpm.get();
    let rate_limiter = Arc::new(RateLimiterState::new(rate_limit_config));
    info!("Initialized rate limiter: {} req/min (API), {} req/min (login)",
        api_rpm, login_rpm);

    // Initialize audit logger
    let audit_logger = Arc::new(AuditLogger::default());
    info!("Initialized audit logger (max 10000 entries in memory)");

    // Initialize config confirmation
    let config_confirmation = Arc::new(ConfigConfirmation::new());
    info!("Initialized config confirmation system");

    // Initialize backup manager
    let backup_config = BackupConfig {
        db_path: config.store.path.clone().into(),
        backup_dir: std::path::PathBuf::from("./backups"),
        retention_count: 7,
        compress: true,
        interval_hours: 24,
    };
    let backup_manager = Arc::new(BackupManager::new(backup_config));
    info!("Initialized backup manager");

    // Initialize payment manager
    let payment_data_dir = std::path::PathBuf::from("./data/payments");
    let payment_config = PaymentConfig {
        bitcoin_rpc_url: std::env::var("BITCOIN_RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8332".to_string()),
        bitcoin_rpc_user: std::env::var("BITCOIN_RPC_USER")
            .unwrap_or_else(|_| "bitcoin".to_string()),
        bitcoin_rpc_pass: std::env::var("BITCOIN_RPC_PASS")
            .unwrap_or_default(),
        ..Default::default()
    };
    let payment_manager = Arc::new(PaymentManager::new(payment_data_dir, payment_config)?);
    payment_manager.load().await?;
    info!("Initialized payment manager");

    // Initialize 2FA manager
    let two_factor_storage = std::path::PathBuf::from("./data/two_factor");
    let two_factor_manager = Arc::new(TwoFactorManager::new(
        two_factor_storage,
        "DMPool Admin".to_string(),
    ));
    two_factor_manager.initialize().await?;
    info!("Initialized 2FA manager");

    let state = AdminState {
        config_path,
        config: Arc::new(RwLock::new(config.clone())),
        store: store.clone(),
        chain_store,
        health_checker: Arc::new(HealthChecker::new(config).with_store(store.clone())),
        auth_manager: auth_manager.clone(),
        two_factor_manager: two_factor_manager.clone(),
        rate_limiter: rate_limiter.clone(),
        audit_logger: audit_logger.clone(),
        config_confirmation: config_confirmation.clone(),
        backup_manager: backup_manager.clone(),
        payment_manager: payment_manager.clone(),
        start_time: std::time::Instant::now(),
        banned_workers: Arc::new(RwLock::new(HashSet::new())),
        worker_tags: Arc::new(RwLock::new(HashMap::new())),
    };

    // Create public router (no auth required, but rate limited)
    let public_routes = Router::new()
        .route("/", get(index))
        .route("/observer", get(observer_search_page))
        .route("/observer/:address", get(observer_page))
        .route("/api/health", get(health))
        .route("/api/services/status", get(services_status))
        .route("/api/observer/:address", get(observer_api))
        .route("/api/observer/:address/shares", get(observer_shares_api))
        .route("/api/observer/:address/payouts", get(observer_payouts_api))
        // Login endpoints (stricter rate limiting)
        .route("/api/auth/login", post(login))
        .route("/api/auth/login2fa", post(login_with_2fa))
        .route_layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ))
        // Apply login-specific rate limiter to login route
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            login_rate_limit_middleware,
        ));

    // Create protected router (auth required + rate limited)
    let protected_routes = Router::new()
        .route("/api/dashboard", get(dashboard))
        .route("/api/config", get(get_config).post(update_config))
        .route("/api/config/reload", post(reload_config))
        .route("/api/workers", get(workers_list))
        .route("/api/workers/:address", get(worker_detail))
        .route("/api/workers/:address/ban", post(ban_worker))
        .route("/api/workers/:address/unban", post(unban_worker))
        .route("/api/workers/:address/tags", post(add_worker_tag))
        .route("/api/workers/:address/tags/:tag", post(remove_worker_tag))
        .route("/api/blocks", get(blocks_list))
        .route("/api/blocks/:height", get(block_detail))
        .route("/api/logs", get(logs))
        .route("/api/safety/check", get(safety_check))
        .route("/api/audit/logs", get(audit_logs))
        .route("/api/audit/stats", get(audit_stats))
        .route("/api/audit/rotate", post(audit_rotate))
        .route("/api/audit/export", post(audit_export))
        .route("/api/config/confirmations", get(get_confirmations))
        .route("/api/config/confirmations/:id", post(confirm_config))
        .route("/api/config/confirmations/:id/apply", post(apply_config))
        // Backup API routes
        .route("/api/backup/create", post(create_backup))
        .route("/api/backup/list", get(list_backups))
        .route("/api/backup/stats", get(backup_stats))
        .route("/api/backup/:id", get(get_backup))
        // 2FA API routes
        .route("/api/2fa/setup", post(two_factor_setup))
        .route("/api/2fa/enable", post(two_factor_enable))
        .route("/api/2fa/disable", post(two_factor_disable))
        .route("/api/2fa/status", get(two_factor_status))
        .route("/api/2fa/verify", post(two_factor_verify))
        .route("/api/backup/:id/delete", post(delete_backup))
        .route("/api/backup/:id/restore", post(restore_backup))
        .route("/api/backup/cleanup", post(cleanup_backups))
        // Payment API routes
        .route("/api/payments/stats", get(payment_stats))
        .route("/api/payments/balances", get(payment_balances))
        .route("/api/payments/balances/:address", get(payment_balance_detail))
        .route("/api/payments/payouts", get(payment_payouts))
        .route("/api/payments/payouts/:address", get(payment_address_payouts))
        .route("/api/payments/create", post(create_payout))
        .route("/api/payments/pending", get(pending_payouts))
        .route("/api/payments/broadcast/:id", post(broadcast_payout))
        .route("/api/payments/config", get(get_payment_config))
        .route("/api/payments/config", post(update_payment_config))
        // Apply rate limiting first
        .route_layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ))
        // Then apply auth middleware
        .route_layer(middleware::from_fn_with_state(
            auth_manager.clone(),
            auth_middleware,
        ));

    // Combine all routes
    let app = public_routes
        .merge(protected_routes)
        .with_state(state)
        .fallback(not_found);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("DMPool Admin Server listening on port {}", port);
    info!("Access admin panel at http://localhost:{}", port);
    info!("Default credentials: {} / {}", admin_username, "***");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Authentication middleware for protected routes
async fn auth_middleware(
    State(auth): State<Arc<AuthManager>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header from request
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header[7..];
            match auth.verify_token(token) {
                Ok(_claims) => {
                    // Token valid, proceed
                    return Ok(next.run(req).await);
                }
                Err(e) => {
                    warn!("Invalid token: {}", e);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    // Allow public routes without auth
    let path = req.uri().path();
    let public_routes = [
        "/",
        "/api/health",
        "/api/services/status",
        "/api/auth/login",
    ];

    if public_routes.iter().any(|r| path == *r || path.starts_with(r)) {
        return Ok(next.run(req).await);
    }

    warn!("Unauthorized access attempt to: {}", path);
    Err(StatusCode::UNAUTHORIZED)
}

/// Serve admin panel index
async fn index() -> impl IntoResponse {
    let html = include_str!("../../static/admin/index.html");
    Html(html)
}

/// Health check
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "dmpool-admin"
    }))
}

/// Get comprehensive services status
async fn services_status(State(state): State<AdminState>) -> impl IntoResponse {
    let health_status = state.health_checker.check().await;
    Json(ApiResponse::ok(health_status))
}

/// Get dashboard metrics
async fn dashboard(State(state): State<AdminState>) -> impl IntoResponse {
    let height = state.chain_store.get_tip_height()
        .ok()
        .flatten()
        .map(|h| h as u64)
        .unwrap_or(0);

    let metrics = DashboardMetrics {
        pool_hashrate_ths: 0.0,
        active_workers: 0,
        total_shares: 0,
        blocks_found: height,
        uptime_seconds: state.start_time.elapsed().as_secs(),
        pplns_window_shares: 0,
        current_difficulty: 1.0,
    };

    Json(ApiResponse::ok(metrics))
}

/// Get current configuration
async fn get_config(State(state): State<AdminState>) -> impl IntoResponse {
    let config = state.config.read().await;

    let view = ConfigView {
        stratum_port: config.stratum.port,
        stratum_hostname: config.stratum.hostname.clone(),
        start_difficulty: config.stratum.start_difficulty,
        minimum_difficulty: config.stratum.minimum_difficulty,
        pplns_ttl_days: config.store.pplns_ttl_days,
        difficulty_multiplier: 1.0,
        network: config.stratum.network.to_string(),
        pool_signature: config.stratum.pool_signature.clone(),
        ignore_difficulty: config.stratum.ignore_difficulty.unwrap_or(false),
        donation: config.stratum.donation,
        fee: None,
    };

    Json(ApiResponse::ok(view))
}

/// Update configuration (runtime only)
async fn update_config(
    State(state): State<AdminState>,
    Json(update): Json<ConfigUpdate>,
) -> impl IntoResponse {
    let mut config = state.config.write().await;
    let mut changes = Vec::new();

    // Update start_difficulty
    if let Some(diff) = update.start_difficulty {
        if diff >= 8 && diff <= 512 {
            let old = config.stratum.start_difficulty;
            config.stratum.start_difficulty = diff as u64;
            changes.push(format!("start_difficulty: {} → {}", old, diff));
            info!("Updated start_difficulty to {}", diff);
        }
    }

    // Update minimum_difficulty
    if let Some(diff) = update.minimum_difficulty {
        if diff >= 8 && diff <= 256 {
            let old = config.stratum.minimum_difficulty;
            config.stratum.minimum_difficulty = diff as u64;
            changes.push(format!("minimum_difficulty: {} → {}", old, diff));
            info!("Updated minimum_difficulty to {}", diff);
        }
    }

    // Update pool_signature
    if let Some(signature) = update.pool_signature {
        if signature.len() <= 16 {
            let old = config.stratum.pool_signature.clone();
            config.stratum.pool_signature = Some(signature.clone());
            changes.push(format!("pool_signature: {:?} → {}", old, signature));
            info!("Updated pool_signature to {}", signature);
        }
    }

    if changes.is_empty() {
        return Json(ApiResponse::<serde_json::Value>::error("No valid changes to apply".to_string()));
    }

    let response = serde_json::json!({
        "message": format!("Applied {} change(s)", changes.len()),
        "changes": changes,
    });

    Json(ApiResponse::ok(response))
}

/// Reload configuration from file
async fn reload_config(State(state): State<AdminState>) -> impl IntoResponse {
    match Config::load(&state.config_path) {
        Ok(new_config) => {
            *state.config.write().await = new_config;
            info!("Configuration reloaded from file");
            let response = serde_json::json!({
                "message": "Configuration reloaded successfully"
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => {
            error!("Failed to reload config: {}", e);
            Json(ApiResponse::<serde_json::Value>::error(format!("Failed to reload: {}", e)))
        }
    }
}

/// Get workers list from PPLNS shares (with pagination)
async fn workers_list(
    State(state): State<AdminState>,
    Query(params): Query<PaginationRequest>,
) -> impl IntoResponse {
    let banned = state.banned_workers.read().await;
    let worker_tags = state.worker_tags.read().await;

    // Get pagination parameters
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let search = params.search.unwrap_or_default().to_lowercase();
    let status_filter = params.status.unwrap_or_default().to_lowercase();

    // Get recent PPLNS shares (last 1000, last 24 hours)
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start_time = end_time - (24 * 3600); // Last 24 hours

    let shares = state.store.get_pplns_shares_filtered(
        Some(1000),
        Some(start_time),
        Some(end_time),
    );

    // Group shares by miner address
    let mut workers_map: HashMap<String, WorkerInfo> = HashMap::new();

    for share in shares {
        let address = share.btcaddress.clone().unwrap_or_else(|| format!("user_{}", share.user_id));

        let entry = workers_map.entry(address.clone()).or_insert_with(|| {
            let now = chrono::Utc::now();
            let is_banned = banned.contains(&address);
            let tags = worker_tags.get(&address).cloned().unwrap_or_default();
            WorkerInfo {
                address: address.clone(),
                worker_name: share.workername.clone().unwrap_or_else(|| "worker".to_string()),
                hashrate_ths: 0.0,
                shares_count: 0,
                difficulty: share.difficulty,
                last_seen: now.to_rfc3339(),
                first_seen: now.to_rfc3339(),
                is_banned,
                tags,
                status: if is_banned {
                    WorkerStatus::Banned
                } else {
                    WorkerStatus::Active
                },
            }
        });

        entry.shares_count += 1;
        entry.difficulty = share.difficulty;
        entry.last_seen = chrono::Utc::now().to_rfc3339();
    }

    // Convert to vector and apply filters
    let mut workers: Vec<WorkerInfo> = workers_map.into_values().collect();

    // Apply search filter
    if !search.is_empty() {
        workers.retain(|w| {
            w.address.to_lowercase().contains(&search)
                || w.worker_name.to_lowercase().contains(&search)
        });
    }

    // Apply status filter
    if !status_filter.is_empty() {
        workers.retain(|w| match status_filter.as_str() {
            "active" => matches!(w.status, WorkerStatus::Active),
            "banned" => matches!(w.status, WorkerStatus::Banned),
            "inactive" => matches!(w.status, WorkerStatus::Inactive),
            _ => true,
        });
    }

    // Apply sorting
    let sort_by = params.sort_by.unwrap_or_else(|| "last_seen".to_string());
    let sort_desc = params.sort_order.unwrap_or_else(|| "desc".to_string()) == "desc";

    match sort_by.as_str() {
        "address" => workers.sort_by(|a, b| {
            if sort_desc {
                b.address.cmp(&a.address)
            } else {
                a.address.cmp(&b.address)
            }
        }),
        "hashrate" => workers.sort_by(|a, b| {
            if sort_desc {
                b.hashrate_ths.partial_cmp(&a.hashrate_ths).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.hashrate_ths.partial_cmp(&b.hashrate_ths).unwrap_or(std::cmp::Ordering::Equal)
            }
        }),
        "shares" => workers.sort_by(|a, b| {
            if sort_desc {
                b.shares_count.cmp(&a.shares_count)
            } else {
                a.shares_count.cmp(&b.shares_count)
            }
        }),
        _ => { // default: last_seen
            workers.sort_by(|a, b| {
                if sort_desc {
                    b.last_seen.cmp(&a.last_seen)
                } else {
                    a.last_seen.cmp(&b.last_seen)
                }
            });
        }
    }

    let total = workers.len();
    let total_pages = (total + page_size - 1) / page_size;

    // Apply pagination
    let start_idx = (page - 1) * page_size;
    let _end_idx = start_idx + page_size;
    let paginated_workers: Vec<WorkerInfo> = workers
        .into_iter()
        .skip(start_idx)
        .take(page_size)
        .collect();

    let response = PaginatedResponse {
        data: paginated_workers,
        total,
        page,
        page_size,
        total_pages,
    };

    Json(ApiResponse::ok(response))
}

/// Get worker detail
async fn worker_detail(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    // Get shares for the specific address
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start_time = end_time - (24 * 3600);

    let all_shares = state.store.get_pplns_shares_filtered(
        Some(1000),
        Some(start_time),
        Some(end_time),
    );

    // Filter shares for the specific address
    let shares: Vec<_> = all_shares
        .into_iter()
        .filter(|s| s.btcaddress.as_ref().map_or(false, |addr| addr == &address))
        .collect();

    if shares.is_empty() {
        return Json(ApiResponse::<serde_json::Value>::error(format!("No shares found for address {} in last 24 hours", address)));
    }

    // Group by worker name
    let mut worker_stats: HashMap<String, u64> = HashMap::new();
    let mut total_shares = 0u64;

    for share in shares {
        let worker = share.workername.clone().unwrap_or_else(|| "worker".to_string());
        *worker_stats.entry(worker).or_insert(0) += 1;
        total_shares += 1;
    }

    let response = serde_json::json!({
        "address": address,
        "total_shares": total_shares,
        "worker_stats": worker_stats,
    });

    Json(ApiResponse::ok(response))
}

/// Ban worker
async fn ban_worker(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Json(req): Json<BanRequest>,
) -> impl IntoResponse {
    state.banned_workers.write().await.insert(address.clone());
    info!("Banned worker: {} - reason: {:?}", address, req.reason);

    let response = serde_json::json!({
        "address": address,
        "banned": true,
        "message": "Worker banned successfully"
    });

    Json(ApiResponse::ok(response))
}

/// Unban worker
async fn unban_worker(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    state.banned_workers.write().await.remove(&address);
    info!("Unbanned worker: {}", address);

    let response = serde_json::json!({
        "address": address,
        "banned": false,
        "message": "Worker unbanned successfully"
    });

    Json(ApiResponse::ok(response))
}

/// Add tag to worker
#[derive(Deserialize)]
struct AddTagRequest {
    tag: String,
}

async fn add_worker_tag(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Json(req): Json<AddTagRequest>,
) -> impl IntoResponse {
    let mut worker_tags = state.worker_tags.write().await;
    let tags = worker_tags.entry(address.clone()).or_insert_with(Vec::new);

    if !tags.contains(&req.tag) {
        tags.push(req.tag.clone());
        info!("Added tag '{}' to worker: {}", req.tag, address);
    }

    let response = serde_json::json!({
        "address": address,
        "tag": req.tag,
        "tags": tags.clone(),
        "message": "Tag added successfully"
    });

    Json(ApiResponse::ok(response))
}

/// Remove tag from worker
async fn remove_worker_tag(
    State(state): State<AdminState>,
    Path((address, tag)): Path<(String, String)>,
) -> impl IntoResponse {
    let mut worker_tags = state.worker_tags.write().await;

    if let Some(tags) = worker_tags.get_mut(&address) {
        let original_len = tags.len();
        tags.retain(|t| t != &tag);
        if tags.len() < original_len {
            info!("Removed tag '{}' from worker: {}", tag, address);
        }
    }

    let current_tags = worker_tags.get(&address).cloned().unwrap_or_default();

    let response = serde_json::json!({
        "address": address,
        "tag": tag,
        "tags": current_tags,
        "message": "Tag removed successfully"
    });

    Json(ApiResponse::ok(response))
}

/// Get blocks list
async fn blocks_list(State(state): State<AdminState>) -> impl IntoResponse {
    let _height = state.chain_store.get_tip_height()
        .ok()
        .flatten()
        .map(|h| h as u64)
        .unwrap_or(0);
    // Return basic info - TODO: Get actual blocks from database
    let blocks: Vec<()> = vec![];
    Json(ApiResponse::ok(blocks))
}

/// Get block detail
async fn block_detail(
    State(_state): State<AdminState>,
    Path(height): Path<String>,
) -> impl IntoResponse {
    let _height: u64 = match height.parse() {
        Ok(h) => h,
        Err(_) => return Json(ApiResponse::<serde_json::Value>::error("Invalid block height".to_string())),
    };
    // TODO: Get actual block detail
    Json(ApiResponse::<serde_json::Value>::error("Block detail not yet implemented".to_string()))
}

/// Get logs
async fn logs(State(_state): State<AdminState>) -> impl IntoResponse {
    // TODO: Return actual log entries
    let logs = vec![
        "2026-02-03 10:00:00 [INFO] DMPool started".to_string(),
        "2026-02-03 10:00:05 [INFO] Connected to Bitcoin RPC".to_string(),
    ];
    Json(ApiResponse::ok(logs))
}

/// Safety check endpoint
async fn safety_check(State(state): State<AdminState>) -> impl IntoResponse {
    let config = state.config.read().await;
    let mut critical = vec![];
    let mut warnings = vec![];

    // Check ignore_difficulty
    if config.stratum.ignore_difficulty.unwrap_or(false) {
        critical.push(SafetyIssue {
            severity: "critical".to_string(),
            param: "ignore_difficulty".to_string(),
            message: "已禁用难度验证，可能导致不公平的PPLNS收益分配".to_string(),
            recommendation: "设置为 false".to_string(),
        });
    }

    // Check pplns_ttl_days
    if config.store.pplns_ttl_days < 7 {
        critical.push(SafetyIssue {
            severity: "critical".to_string(),
            param: "pplns_ttl_days".to_string(),
            message: format!(
                "TTL={}天过短，标准为7天，矿工可能损失约{}%的收益",
                config.store.pplns_ttl_days,
                ((7 - config.store.pplns_ttl_days) * 100 / 7)
            ),
            recommendation: "设置为 7".to_string(),
        });
    }

    // Check donation
    if let Some(donation) = config.stratum.donation {
        if donation >= 10000 {
            critical.push(SafetyIssue {
                severity: "critical".to_string(),
                param: "donation".to_string(),
                message: "donation=10000意味着100%捐赠，矿工收益为0！".to_string(),
                recommendation: "设置为0或注释掉donation".to_string(),
            });
        } else if donation > 500 {
            warnings.push(SafetyIssue {
                severity: "warning".to_string(),
                param: "donation".to_string(),
                message: format!("捐赠比例较高: {}%", donation / 100),
                recommendation: "考虑设置为0-500(0-5%)".to_string(),
            });
        }
    }

    let safe = critical.is_empty();

    Json(SafetyReport {
        safe,
        critical_issues: critical,
        warnings,
    })
}

/// Login endpoint using AdminState
async fn login(
    State(state): State<AdminState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    match state.auth_manager.authenticate(&req.username, &req.password).await {
        Ok(Some(user)) => {
            let token = state.auth_manager.generate_token(&user)
                .map_err(|e| {
                    error!("Failed to generate token: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            let expires_in = 24 * 3600; // 24 hours

            info!("User '{}' logged in successfully", req.username);

            Ok(Json(LoginResponse {
                token,
                user_info: UserInfo {
                    username: user.username,
                    role: user.role,
                },
                expires_in,
            }))
        }
        Ok(None) => {
            warn!("Failed login attempt for user '{}'", req.username);
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(e) => {
            error!("Authentication error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get audit logs
async fn audit_logs(
    State(state): State<AdminState>,
    Query(filter): Query<AuditFilterWrapper>,
) -> impl IntoResponse {
    let logs = state.audit_logger.query(filter.0).await;
    Json(ApiResponse::ok(logs))
}

/// Get audit statistics
async fn audit_stats(State(state): State<AdminState>) -> impl IntoResponse {
    let stats = state.audit_logger.stats().await;
    Json(ApiResponse::ok(stats))
}

/// Rotate audit logs
async fn audit_rotate(State(state): State<AdminState>) -> impl IntoResponse {
    match state.audit_logger.rotate_logs().await {
        Ok(archive_path) => {
            let response = serde_json::json!({
                "message": "Audit logs rotated successfully",
                "archive_file": archive_path
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to rotate logs: {}",
            e
        ))),
    }
}

/// Export audit logs
async fn audit_export(State(state): State<AdminState>) -> impl IntoResponse {
    let output_path = std::path::PathBuf::from(format!(
        "./audit_export_{}.jsonl",
        Utc::now().format("%Y%m%d_%H%M%S")
    ));

    match state.audit_logger.export(output_path.clone()).await {
        Ok(count) => {
            let response = serde_json::json!({
                "message": format!("Exported {} audit log entries", count),
                "file": output_path
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to export logs: {}",
            e
        ))),
    }
}

/// Wrapper for Query<AuditFilter> to implement FromRequest
#[derive(Debug, Deserialize)]
struct AuditFilterWrapper(AuditFilter);

impl Default for AuditFilterWrapper {
    fn default() -> Self {
        Self(AuditFilter::default())
    }
}

/// Get pending configuration change confirmations
async fn get_confirmations(State(state): State<AdminState>) -> impl IntoResponse {
    let pending = state.config_confirmation.get_pending().await;
    Json(ApiResponse::ok(pending))
}

/// Request a configuration change (creates confirmation request)
async fn request_config_change(
    State(state): State<AdminState>,
    Json(req): Json<ConfigChangeRequestData>,
) -> impl IntoResponse {
    // Validate the new value
    if let Err(e) = state
        .config_confirmation
        .validate_value(&req.parameter, &req.new_value)
    {
        return Json(ApiResponse::<serde_json::Value>::error(format!(
            "Invalid value for {}: {}",
            req.parameter, e
        )));
    }

    // Check if confirmation is required
    if !state
        .config_confirmation
        .requires_confirmation(&req.parameter)
    {
        // Apply immediately if no confirmation needed
        let response = serde_json::json!({
            "message": format!("{} updated (no confirmation required)", req.parameter),
            "parameter": req.parameter,
            "old_value": req.old_value,
            "new_value": req.new_value,
            "confirmed": true,
            "applied": true,
        });
        return Json(ApiResponse::ok(response));
    }

    // Create confirmation request
    match state
        .config_confirmation
        .create_change_request(
            req.parameter.clone(),
            req.old_value,
            req.new_value.clone(),
            req.username.clone(),
            req.ip_address.clone(),
        )
        .await
    {
        Ok(request) => {
            // Get risk level info
            let risk_level = state
                .config_confirmation
                .get_risk_level(&req.parameter);

            let response = serde_json::json!({
                "message": "Confirmation required for this change",
                "request": request,
                "risk_level": risk_level,
                "meta": state.config_confirmation.get_config_meta(&req.parameter),
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to create confirmation request: {}",
            e
        ))),
    }
}

/// Confirm a pending configuration change
async fn confirm_config(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_confirmation.confirm_change(&id).await {
        Ok(true) => {
            let response = serde_json::json!({
                "message": "Change confirmed. Use /apply to apply the change.",
                "id": id
            });
            Json(ApiResponse::ok(response))
        }
        Ok(false) => {
            Json(ApiResponse::<serde_json::Value>::error(
                "Change request not found or expired".to_string(),
            ))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to confirm change: {}",
            e
        ))),
    }
}

/// Apply a confirmed configuration change
async fn apply_config(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_confirmation.apply_change(&id).await {
        Ok(request) => {
            // TODO: Actually apply the config change to the running config
            // For now, just log it

            let response = serde_json::json!({
                "message": format!("Config change applied: {} = {:?}", request.parameter, request.new_value),
                "request": request
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to apply change: {}",
            e
        ))),
    }
}

// ===== Backup API Handlers =====

/// Create a new backup
async fn create_backup(State(state): State<AdminState>) -> impl IntoResponse {
    match state.backup_manager.create_backup().await {
        Ok(metadata) => {
            let response = serde_json::json!({
                "message": "Backup created successfully",
                "backup": metadata
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to create backup: {}",
            e
        ))),
    }
}

/// List all backups
async fn list_backups(State(state): State<AdminState>) -> impl IntoResponse {
    match state.backup_manager.list_backups() {
        Ok(backups) => {
            let response = serde_json::json!({
                "backups": backups,
                "count": backups.len()
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to list backups: {}",
            e
        ))),
    }
}

/// Get backup statistics
async fn backup_stats(State(state): State<AdminState>) -> impl IntoResponse {
    match state.backup_manager.get_stats() {
        Ok(stats) => {
            let response = serde_json::json!({
                "stats": stats
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to get backup stats: {}",
            e
        ))),
    }
}

/// Get a specific backup by ID
async fn get_backup(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.backup_manager.load_metadata(&id) {
        Ok(metadata) => {
            let response = serde_json::json!({
                "backup": metadata
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to load backup: {}",
            e
        ))),
    }
}

/// Delete a backup
async fn delete_backup(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.backup_manager.delete_backup(&id).await {
        Ok(_) => {
            let response = serde_json::json!({
                "message": format!("Backup {} deleted successfully", id)
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to delete backup: {}",
            e
        ))),
    }
}

/// Restore from a backup
async fn restore_backup(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.backup_manager.restore_backup(&id, None).await {
        Ok(_) => {
            let response = serde_json::json!({
                "message": format!("Backup {} restored successfully", id),
                "note": "Database service restart may be required"
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to restore backup: {}",
            e
        ))),
    }
}

/// Cleanup old backups based on retention policy
async fn cleanup_backups(State(state): State<AdminState>) -> impl IntoResponse {
    match state.backup_manager.cleanup_old_backups().await {
        Ok(count) => {
            let response = serde_json::json!({
                "message": format!("Cleaned up {} old backup(s)", count),
                "deleted_count": count
            });
            Json(ApiResponse::ok(response))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!(
            "Failed to cleanup backups: {}",
            e
        ))),
    }
}

/// Data for creating a config change request
#[derive(Deserialize)]
struct ConfigChangeRequestData {
    pub parameter: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub username: String,
    pub ip_address: String,
}

/// Observer API - Get public stats for a Bitcoin address
async fn observer_api(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    // Validate Bitcoin address format (basic validation)
    if address.len() < 26 || address.len() > 90 {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Invalid Bitcoin address format"
        }));
    }

    // Get shares for the specific address (last 24 hours)
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start_time = end_time - (24 * 3600);

    let all_shares = state.store.get_pplns_shares_filtered(
        Some(5000),
        Some(start_time),
        Some(end_time),
    );

    // Filter shares for the specific address
    let shares: Vec<_> = all_shares
        .into_iter()
        .filter(|s| s.btcaddress.as_ref().map_or(false, |addr| addr == &address))
        .collect();

    if shares.is_empty() {
        return Json(serde_json::json!({
            "status": "ok",
            "data": {
                "address": address,
                "total_shares": 0,
                "workers": {},
                "hashrate_ths": 0.0,
                "last_share": null,
                "first_share": null,
                "period_hours": 24
            },
            "message": "No shares found for this address in the last 24 hours"
        }));
    }

    // Group by worker name and calculate stats
    let mut worker_stats: HashMap<String, serde_json::Value> = HashMap::new();
    let mut total_shares = 0u64;
    let mut total_difficulty = 0u64;
    let mut first_timestamp = u64::MAX;
    let mut last_timestamp = 0u64;

    for share in &shares {
        let worker = share.workername.clone().unwrap_or_else(|| "worker".to_string());
        total_shares += 1;
        total_difficulty += share.difficulty;
        let n_time = share.n_time;
        first_timestamp = first_timestamp.min(n_time);
        last_timestamp = last_timestamp.max(n_time);

        let entry = worker_stats.entry(worker.clone()).or_insert_with(|| {
            serde_json::json!({
                "name": worker,
                "shares": 0u64,
                "difficulty": 0u64,
                "first_seen": n_time,
                "last_seen": n_time,
            })
        });

        if let Some(obj) = entry.as_object_mut() {
            *obj.get_mut("shares").unwrap() = serde_json::Value::Number(
                serde_json::Number::from(obj["shares"].as_u64().unwrap_or(0) + 1)
            );
            *obj.get_mut("difficulty").unwrap() = serde_json::Value::Number(
                serde_json::Number::from(obj["difficulty"].as_u64().unwrap_or(0) + share.difficulty)
            );
            *obj.get_mut("last_seen").unwrap() = serde_json::Value::Number(
                serde_json::Number::from(n_time)
            );
        }
    }

    // Calculate estimated hashrate (very rough approximation)
    // Hashrate (TH/s) ≈ (Total Difficulty * 2^32) / (Time Window in seconds * 10^12)
    let time_window = (last_timestamp - first_timestamp).max(3600); // At least 1 hour
    let hashrate_ths = if time_window > 0 {
        (total_difficulty as f64 * 4_294_967_296.0) / (time_window as f64 * 1_000_000_000_000.0)
    } else {
        0.0
    };

    Json(serde_json::json!({
        "status": "ok",
        "data": {
            "address": address,
            "total_shares": total_shares,
            "total_difficulty": total_difficulty,
            "workers": worker_stats,
            "hashrate_ths": hashrate_ths,
            "last_share": last_timestamp,
            "first_share": first_timestamp,
            "period_hours": 24
        }
    }))
}

/// Observer Shares API - Get recent shares for an address
async fn observer_shares_api(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Query(params): Query<SharesQuery>,
) -> impl IntoResponse {
    // Validate address
    if address.len() < 26 || address.len() > 90 {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Invalid Bitcoin address format"
        }));
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    // Get shares for the specific address (last 24 hours)
    let end_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start_time = end_time - (7 * 24 * 3600); // 7 days

    let all_shares = state.store.get_pplns_shares_filtered(
        Some(5000),
        Some(start_time),
        Some(end_time),
    );

    // Filter shares for the specific address
    let mut shares: Vec<_> = all_shares
        .into_iter()
        .filter(|s| s.btcaddress.as_ref().map_or(false, |addr| addr == &address))
        .collect();

    // Reverse to show newest first
    shares.reverse();

    // Apply pagination
    let total = shares.len();
    let shares_paginated: Vec<_> = shares.into_iter()
        .skip(offset)
        .take(limit)
        .map(|share| {
            serde_json::json!({
                "timestamp": share.n_time,
                "difficulty": share.difficulty,
                "worker_name": share.workername.clone().unwrap_or_else(|| "worker".to_string()),
                "user_id": share.user_id
            })
        })
        .collect();

    Json(serde_json::json!({
        "status": "ok",
        "data": {
            "address": address,
            "shares": shares_paginated,
            "total": total,
            "limit": limit,
            "offset": offset
        }
    }))
}

/// Observer Payouts API - Get payout history for an address
async fn observer_payouts_api(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    // Validate address
    if address.len() < 26 || address.len() > 90 {
        return Json(serde_json::json!({
            "status": "error",
            "message": "Invalid Bitcoin address format"
        }));
    }

    // Get payout history from payment manager
    let payouts = state.payment_manager.get_payout_history(&address, 100).await;

    // Calculate totals
    let total_paid_satoshis: u64 = payouts.iter()
        .filter(|p| p.status == PayoutStatus::Confirmed)
        .map(|p| p.amount_satoshis)
        .sum();

    Json(serde_json::json!({
        "status": "ok",
        "data": {
            "address": address,
            "payouts": payouts,
            "total_payouts": payouts.len(),
            "total_paid_satoshis": total_paid_satoshis
        }
    }))
}

/// Query parameters for shares API
#[derive(Deserialize)]
struct SharesQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

/// Observer search page - Public web interface for searching miner stats
async fn observer_search_page() -> impl IntoResponse {
    let html = include_str!("../../static/admin/observer.html");
    Html(html)
}

/// Observer page - Public web interface for viewing miner stats
async fn observer_page(Path(_address): Path<String>) -> impl IntoResponse {
    let html = include_str!("../../static/admin/observer.html");
    Html(html)
}

// ===== Payment API Handlers =====

/// Get payment statistics
async fn payment_stats(State(state): State<AdminState>) -> impl IntoResponse {
    let stats = state.payment_manager.get_stats().await;
    Json(ApiResponse::ok(stats))
}

/// Get all miner balances
async fn payment_balances(
    State(state): State<AdminState>,
    Query(params): Query<PaginationRequest>,
) -> impl IntoResponse {
    let mut balances = state.payment_manager.get_all_balances().await;

    // Apply search filter
    if let Some(search) = params.search {
        let search_lower = search.to_lowercase();
        balances.retain(|b| b.address.to_lowercase().contains(&search_lower));
    }

    // Apply sorting
    let sort_by = params.sort_by.unwrap_or_else(|| "balance".to_string());
    let sort_desc = params.sort_order.unwrap_or_else(|| "desc".to_string()) == "desc";

    match sort_by.as_str() {
        "address" => balances.sort_by(|a, b| {
            if sort_desc {
                b.address.cmp(&a.address)
            } else {
                a.address.cmp(&b.address)
            }
        }),
        "earned" => balances.sort_by(|a, b| {
            if sort_desc {
                b.total_earned_satoshis.cmp(&a.total_earned_satoshis)
            } else {
                a.total_earned_satoshis.cmp(&b.total_earned_satoshis)
            }
        }),
        _ => { // balance (default)
            balances.sort_by(|a, b| {
                if sort_desc {
                    b.balance_satoshis.cmp(&a.balance_satoshis)
                } else {
                    a.balance_satoshis.cmp(&b.balance_satoshis)
                }
            });
        }
    }

    // Apply pagination
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let total = balances.len();
    let total_pages = (total + page_size - 1) / page_size;

    let paginated_balances: Vec<MinerBalance> = balances
        .into_iter()
        .skip((page - 1) * page_size)
        .take(page_size)
        .collect();

    let response = PaginatedResponse {
        data: paginated_balances,
        total,
        page,
        page_size,
        total_pages,
    };

    Json(ApiResponse::ok(response))
}

/// Get specific miner balance
async fn payment_balance_detail(
    State(state): State<AdminState>,
    Path(address): Path<String>,
) -> impl IntoResponse {
    match state.payment_manager.get_balance(&address).await {
        Some(balance) => Json(ApiResponse::ok(balance)),
        None => Json(ApiResponse::<MinerBalance>::error(format!("No balance found for address {}", address))),
    }
}

/// Get all payout records (with pagination)
async fn payment_payouts(
    State(state): State<AdminState>,
    Query(params): Query<PayoutQuery>,
) -> impl IntoResponse {
    let mut result = state.payment_manager.get_all_payouts().await;

    // Filter by status if specified
    if let Some(status) = params.status {
        let payout_status = match status.as_str() {
            "pending" => Some(PayoutStatus::Pending),
            "broadcast" => Some(PayoutStatus::Broadcast),
            "confirmed" => Some(PayoutStatus::Confirmed),
            "failed" => Some(PayoutStatus::Failed),
            _ => None,
        };
        if let Some(ps) = payout_status {
            result.retain(|p| p.status == ps);
        }
    }

    // Reverse to show newest first
    result.reverse();

    // Apply pagination
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).min(100);
    let total = result.len();
    let total_pages = (total + page_size - 1) / page_size;

    let paginated_payouts: Vec<Payout> = result
        .into_iter()
        .skip((page - 1) * page_size)
        .take(page_size)
        .collect();

    let response = PayoutsResponse {
        data: paginated_payouts,
        total,
        page,
        page_size,
        total_pages,
    };

    Json(ApiResponse::ok(response))
}

/// Get payout history for a specific address
async fn payment_address_payouts(
    State(state): State<AdminState>,
    Path(address): Path<String>,
    Query(params): Query<AddressPayoutQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(500);
    let payouts = state.payment_manager.get_payout_history(&address, limit).await;

    Json(ApiResponse::ok(serde_json::json!({
        "address": address,
        "payouts": payouts,
        "total": payouts.len()
    })))
}

/// Create a manual payout
#[derive(Deserialize)]
struct CreatePayoutRequest {
    address: String,
    amount_satoshis: u64,
}

async fn create_payout(
    State(state): State<AdminState>,
    Json(req): Json<CreatePayoutRequest>,
) -> impl IntoResponse {
    match state.payment_manager.create_payout(req.address.clone(), req.amount_satoshis).await {
        Ok(payout) => {
            info!("Created manual payout {} to {} for {} satoshis", payout.id, req.address, req.amount_satoshis);
            Json(ApiResponse::ok(serde_json::json!({
                "payout_id": payout.id,
                "address": payout.address,
                "amount_satoshis": payout.amount_satoshis,
                "status": payout.status,
                "message": "Payout created successfully"
            })))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!("Failed to create payout: {}", e)))
    }
}

/// Get pending payouts
async fn pending_payouts(State(state): State<AdminState>) -> impl IntoResponse {
    let pending = state.payment_manager.get_pending_payout_records().await;

    Json(ApiResponse::ok(serde_json::json!({
        "pending_count": pending.len(),
        "payouts": pending
    })))
}

/// Broadcast a pending payout
async fn broadcast_payout(
    State(state): State<AdminState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.payment_manager.broadcast_payout(&id).await {
        Ok(payout) => {
            info!("Broadcast payout {} to {} for {} satoshis", payout.id, payout.address, payout.amount_satoshis);
            Json(ApiResponse::ok(serde_json::json!({
                "payout_id": payout.id,
                "txid": payout.txid,
                "status": payout.status,
                "message": "Payout broadcast successfully"
            })))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!("Failed to broadcast payout: {}", e)))
    }
}

/// Get payment configuration
async fn get_payment_config(State(state): State<AdminState>) -> impl IntoResponse {
    let config = state.payment_manager.get_config().await;
    Json(ApiResponse::ok(serde_json::json!({
        "min_payout_btc": config.min_payout_satoshis as f64 / 100_000_000.0,
        "manual_payout_btc": config.manual_payout_satoshis as f64 / 100_000_000.0,
        "lightning_payout_btc": config.lightning_payout_satoshis as f64 / 100_000_000.0,
        "required_confirmations": config.required_confirmations,
        "pool_fee_percent": config.pool_fee_bps as f64 / 100.0,
        "donation_percent": config.donation_bps as f64 / 100.0,
        "auto_payout_enabled": config.auto_payout_enabled,
        "auto_payout_interval_hours": config.auto_payout_interval_hours,
        "bitcoin_rpc_url": config.bitcoin_rpc_url
    })))
}

/// Update payment configuration
#[derive(Deserialize)]
struct PaymentConfigUpdate {
    min_payout_satoshis: Option<u64>,
    manual_payout_satoshis: Option<u64>,
    auto_payout_enabled: Option<bool>,
    auto_payout_interval_hours: Option<u32>,
    pool_fee_bps: Option<u32>,
    bitcoin_rpc_url: Option<String>,
    bitcoin_rpc_user: Option<String>,
    bitcoin_rpc_pass: Option<String>,
}

async fn update_payment_config(
    State(state): State<AdminState>,
    Json(update): Json<PaymentConfigUpdate>,
) -> impl IntoResponse {
    let mut config = state.payment_manager.get_config().await;

    if let Some(min) = update.min_payout_satoshis {
        config.min_payout_satoshis = min;
    }
    if let Some(manual) = update.manual_payout_satoshis {
        config.manual_payout_satoshis = manual;
    }
    if let Some(enabled) = update.auto_payout_enabled {
        config.auto_payout_enabled = enabled;
    }
    if let Some(interval) = update.auto_payout_interval_hours {
        config.auto_payout_interval_hours = interval;
    }
    if let Some(fee) = update.pool_fee_bps {
        config.pool_fee_bps = fee;
    }
    if let Some(url) = update.bitcoin_rpc_url {
        config.bitcoin_rpc_url = url;
    }
    if let Some(user) = update.bitcoin_rpc_user {
        config.bitcoin_rpc_user = user;
    }
    if let Some(pass) = update.bitcoin_rpc_pass {
        config.bitcoin_rpc_pass = pass;
    }

    match state.payment_manager.update_config(config).await {
        Ok(_) => {
            info!("Payment configuration updated");
            Json(ApiResponse::ok(serde_json::json!({
                "message": "Payment configuration updated successfully"
            })))
        }
        Err(e) => Json(ApiResponse::<serde_json::Value>::error(format!("Failed to update config: {}", e)))
    }
}

/// Query parameters for payout listing
#[derive(Deserialize)]
struct PayoutQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    status: Option<String>,
}

/// Query parameters for address payouts
#[derive(Deserialize)]
struct AddressPayoutQuery {
    limit: Option<usize>,
}

/// Response for payouts list
#[derive(Serialize)]
struct PayoutsResponse {
    data: Vec<Payout>,
    total: usize,
    page: usize,
    page_size: usize,
    total_pages: usize,
}

// ===== 2FA Login Endpoint =====

/// Login request with 2FA support
#[derive(Deserialize)]
struct LoginRequest2FA {
    pub username: String,
    pub password: String,
    pub totp_code: Option<String>,
    pub backup_code: Option<String>,
}

/// Login response with 2FA support
#[derive(Serialize)]
struct LoginResponse2FA {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_info: Option<UserInfo>,
    pub requires_2fa: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Login endpoint with 2FA support
async fn login_with_2fa(
    State(state): State<AdminState>,
    Json(req): Json<LoginRequest2FA>,
) -> Result<Json<LoginResponse2FA>, StatusCode> {
    // Step 1: Authenticate username and password
    let user = match state.auth_manager.authenticate(&req.username, &req.password).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            warn!("Failed login attempt for user '{}'", req.username);
            return Err(StatusCode::UNAUTHORIZED);
        }
        Err(e) => {
            error!("Authentication error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Step 2: Check if 2FA is enabled for this user
    let two_fa_status = state.two_factor_manager.get_status(&req.username).await;
    let requires_2fa = two_fa_status.enabled;

    if !requires_2fa {
        // No 2FA required, generate token
        let token = state.auth_manager.generate_token(&user).map_err(|e| {
            error!("Failed to generate token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        info!("User '{}' logged in successfully (no 2FA)", req.username);

        return Ok(Json(LoginResponse2FA {
            token: Some(token),
            user_info: Some(UserInfo {
                username: user.username,
                role: user.role,
            }),
            requires_2fa: false,
            message: None,
        }));
    }

    // Step 3: 2FA is required, verify the code
    let totp_code = req.totp_code.as_deref().unwrap_or("");
    let backup_code = req.backup_code.as_deref();

    match state.two_factor_manager.verify_login(
        &req.username,
        if totp_code.is_empty() { None } else { Some(totp_code) },
        backup_code,
    ).await {
        Ok(true) => {
            // 2FA verification successful
            let token = state.auth_manager.generate_token(&user).map_err(|e| {
                error!("Failed to generate token: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            info!("User '{}' logged in successfully with 2FA", req.username);

            Ok(Json(LoginResponse2FA {
                token: Some(token),
                user_info: Some(UserInfo {
                    username: user.username,
                    role: user.role,
                }),
                requires_2fa: false,
                message: None,
            }))
        }
        Ok(false) => {
            warn!("Failed 2FA verification for user '{}'", req.username);
            Ok(Json(LoginResponse2FA {
                token: None,
                user_info: None,
                requires_2fa: true,
                message: Some("Invalid 2FA code".to_string()),
            }))
        }
        Err(e) => {
            error!("2FA verification error for user '{}': {}", req.username, e);
            Ok(Json(LoginResponse2FA {
                token: None,
                user_info: None,
                requires_2fa: true,
                message: Some(format!("2FA error: {}", e)),
            }))
        }
    }
}

// ===== 2FA API Endpoints =====

/// 2FA setup response
#[derive(Serialize)]
struct TwoFactorSetupResponse {
    requires_2fa: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    setup_data: Option<serde_json::Value>,
}

/// Setup 2FA for a user (requires auth)
async fn two_factor_setup(
    State(state): State<AdminState>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");

    if username.is_empty() {
        return Json(ApiResponse::<serde_json::Value>::error("Username is required"));
    }

    match state.two_factor_manager.generate_secret(username).await {
        Ok(setup) => {
            info!("2FA setup initiated for user '{}'", username);
            let setup_value = serde_json::to_value(setup).unwrap_or(serde_json::json!({}));
            let response = TwoFactorSetupResponse {
                requires_2fa: false,
                setup_data: Some(setup_value),
            };
            Json(ApiResponse::ok(serde_json::to_value(response).unwrap_or_default()))
        }
        Err(e) => {
            error!("Failed to generate 2FA secret: {}", e);
            Json(ApiResponse::<serde_json::Value>::error(format!("Failed to setup 2FA: {}", e)))
        }
    }
}

/// Enable 2FA for a user
async fn two_factor_enable(
    State(state): State<AdminState>,
    Json(req): Json<TwoFactorEnable>,
) -> impl IntoResponse {
    match state.two_factor_manager.enable_2fa(&req.username, &req.code).await {
        Ok(_) => {
            info!("2FA enabled for user '{}'", req.username);
            Json(ApiResponse::ok(serde_json::json!({
                "message": "2FA enabled successfully",
                "username": req.username
            })))
        }
        Err(e) => {
            warn!("Failed to enable 2FA for user '{}': {}", req.username, e);
            Json(ApiResponse::<serde_json::Value>::error(format!("Failed to enable 2FA: {}", e)))
        }
    }
}

/// Disable 2FA for a user
async fn two_factor_disable(
    State(state): State<AdminState>,
    Json(req): Json<TwoFactorEnable>,
) -> impl IntoResponse {
    match state.two_factor_manager.disable_2fa(&req.username).await {
        Ok(_) => {
            info!("2FA disabled for user '{}'", req.username);
            Json(ApiResponse::ok(serde_json::json!({
                "message": "2FA disabled successfully",
                "username": req.username
            })))
        }
        Err(e) => {
            warn!("Failed to disable 2FA for user '{}': {}", req.username, e);
            Json(ApiResponse::<serde_json::Value>::error(format!("Failed to disable 2FA: {}", e)))
        }
    }
}

/// Get 2FA status for a user
async fn two_factor_status(
    State(state): State<AdminState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    let status = state.two_factor_manager.get_status(&username).await;
    Json(ApiResponse::ok(status))
}

/// Verify 2FA code
async fn two_factor_verify(
    State(state): State<AdminState>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let code = req.get("code").and_then(|v| v.as_str());
    let backup_code = req.get("backup_code").and_then(|v| v.as_str());

    match state.two_factor_manager.verify_login(username, code, backup_code).await {
        Ok(true) => {
            info!("2FA verification successful for user '{}'", username);
            Json(ApiResponse::ok(serde_json::json!({
                "valid": true,
                "message": "2FA verification successful"
            })))
        }
        Ok(false) => {
            warn!("2FA verification failed for user '{}'", username);
            Json(ApiResponse::ok(serde_json::json!({
                "valid": false,
                "message": "Invalid 2FA code"
            })))
        }
        Err(e) => {
            error!("2FA verification error for user '{}': {}", username, e);
            Json(ApiResponse::<serde_json::Value>::error(format!("Verification error: {}", e)))
        }
    }
}

/// 404 handler
async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}
