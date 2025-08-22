use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region};
use axum::{
    Router,
    routing::{get, post, put},
};
use deadpool_redis::{Config, Runtime};
use std::{env, sync::Arc};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use websocket::AppWsState;

mod handlers;
pub mod shared_types;
mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub redis: deadpool_redis::Pool,
    pub s3: Client,
    pub ws_state: Arc<AppWsState>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ephemeral_backend=debug,aws_config=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // --- S3 Client Setup ---
    let s3_endpoint_url =
        env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let s3_region = Region::new("us-east-1");
    let region_provider = RegionProviderChain::first_try(s3_region.clone());
    // Build the shared AWS config first.
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(&s3_endpoint_url)
        .load()
        .await;

    // Construct an S3-specific config that forces path-style addressing.
    // This prevents the SDK from using virtual-host style ("{bucket}.{endpoint}")
    // which would try to resolve DNS names like "ephemeral.minio" and fail inside
    // container networks. MinIO running at service name `minio` requires path-style.
    let s3_conf = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(true)
        .build();

    // Create the S3 client from the S3-specific config.
    let s3_client = Client::from_conf(s3_conf);
    info!("Connected to S3-compatible storage.");

    // Ensure the bucket used by the application exists. This is best-effort:
    // if the bucket already exists or cannot be created for some reason,
    // we log the result and continue. This avoids "NoSuchBucket" errors
    // during first-time uploads to a freshly started MinIO instance.
    let bucket_name = "ephemeral";
    match s3_client.create_bucket().bucket(bucket_name).send().await {
        Ok(_) => info!("Ensured S3 bucket '{}' exists.", bucket_name),
        Err(e) => tracing::warn!("Could not create bucket '{}': {:?}", bucket_name, e),
    }

    // --- SETUP REDIS POOL (same as before) ---
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let cfg = Config::from_url(redis_url);
    let redis_pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool.");
    info!("Connected to Redis and created connection pool.");

    // --- WebSocket State Setup ---
    let ws_state = Arc::new(AppWsState::default());

    // --- AppState Setup ---
    let app_state = AppState {
        redis: redis_pool,
        s3: s3_client,
        ws_state,
    };

    // --- CORS Setup ---
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // --- ADD the new routes to the router ---
    let app = Router::new()
        .route("/api/spaces", post(handlers::create_space))
        .route("/api/spaces/{id}", get(handlers::get_space))
        .route("/api/spaces/{id}/text", put(handlers::update_text_bin))
        .route("/api/spaces/{id}/files", post(handlers::upload_file))
        .route("/api/spaces/{id}/download", get(handlers::download_files))
        .route("/ws/spaces/{id}", get(websocket::websocket_handler))
        .with_state(app_state)
        .layer(cors);

    // --- Server Launch ---
        // Bind to 0.0.0.0 so the server is reachable from other hosts/containers
        // (when running inside Docker the process must listen on all interfaces).
        let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
        info!("🚀 Server listening on {}", listener.local_addr().unwrap());
        axum::serve(listener, app).await.unwrap();
}
