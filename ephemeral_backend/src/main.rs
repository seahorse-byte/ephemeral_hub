use axum::{
    Router,
    routing::{get, post, put},
};
use deadpool_redis::{Config, Runtime};
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// --- ADD these new `use` statements ---
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

mod handlers;

// --- UPDATE the AppState type definition ---
#[derive(Clone)]
pub struct AppState {
    pub redis: deadpool_redis::Pool,
    pub s3: Client,
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

    // --- SETUP S3 CLIENT ---
    let s3_endpoint_url =
        env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());
    let s3_region = Region::new("us-east-1");

    let region_provider = RegionProviderChain::first_try(s3_region.clone());

    // --- THIS IS THE FIX for the deprecation warning ---
    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .endpoint_url(&s3_endpoint_url)
        .load()
        .await;

    let s3_client = Client::new(&s3_config);
    info!("Connected to S3-compatible storage.");

    // --- SETUP REDIS POOL (same as before) ---
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
    let cfg = Config::from_url(redis_url);
    let redis_pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool.");
    info!("Connected to Redis and created connection pool.");

    // --- COMBINE into the new AppState ---
    let app_state = AppState {
        redis: redis_pool,
        s3: s3_client,
    };

    // FIX: Create a CORS layer to allow requests from the web frontend.
    // For production, you would restrict this to your specific frontend domain
    // instead of using `Any`.
    let cors = CorsLayer::new()
        .allow_origin(Any) // Allows any origin
        .allow_methods(Any) // Allows any method (GET, POST, etc.)
        .allow_headers(Any); // Allows any header

    // --- ADD the new routes to the router ---
    let app = Router::new()
        .route("/api/spaces", post(handlers::create_space))
        .route("/api/spaces/{id}", get(handlers::get_space))
        .route("/api/spaces/{id}/text", put(handlers::update_text_bin))
        .route("/api/spaces/{id}/files", post(handlers::upload_file))
        .route("/api/spaces/{id}/download", get(handlers::download_files))
        .with_state(app_state)
        .layer(cors);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    info!("🚀 Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
