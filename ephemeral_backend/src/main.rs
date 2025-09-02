// AWS SDK crates
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client as S3Client, config::Region};
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
    pub s3: S3Client,
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
    let aws_region_str = env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let s3_endpoint_url =
        env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:9000".to_string());

    let s3_bucket_name =
        env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set in the environment");

    let region_provider = RegionProviderChain::first_try(Region::new(aws_region_str));

    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(&s3_endpoint_url)
        .load()
        .await;

    let s3_client = S3Client::new(&s3_config);
    info!("Connected to S3-compatible storage.");

    // Verify the application can access the configured bucket.
    if let Err(e) = handlers::verify_bucket_access(&s3_client, &s3_bucket_name).await {
        tracing::error!(
            "Could not verify access to bucket '{}': {:?}",
            s3_bucket_name,
            e
        );
    } else {
        info!(
            "Successfully verified access to S3 bucket '{}'.",
            s3_bucket_name
        );
    }

    // --- SETUP REDIS POOL---
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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/hubs", post(handlers::create_hub))
        .route("/api/hubs/{id}", get(handlers::get_hub))
        .route("/api/hubs/{id}/text", put(handlers::update_text_bin))
        .route("/api/hubs/{id}/files", post(handlers::upload_file))
        .route("/api/hubs/{id}/download", get(handlers::download_files))
        .route("/ws/hubs/{id}", get(websocket::websocket_handler))
        .with_state(app_state)
        .layer(cors);

    // --- Server Launch ---
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("🚀 Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
