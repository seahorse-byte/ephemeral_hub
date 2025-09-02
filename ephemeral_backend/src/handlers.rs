use crate::{AppState, shared_types::PathData};
use aws_sdk_s3::{
    Client as S3Client, error::SdkError, operation::head_bucket::HeadBucketError,
    primitives::ByteStream,
};

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Multipart;
use chrono::{DateTime, Duration, Utc};
use nanoid::nanoid;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Cursor, Write};
use tracing::debug;
use zip::write::{FileOptions, ZipWriter};

#[derive(Debug)]
pub enum AppError {
    PoolError(deadpool_redis::PoolError),
    RedisError(redis::RedisError),
    S3PutError(aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>),
    S3GetError(aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>),
    UploadError(String),
    ZipError(zip::result::ZipError),
    IoError(io::Error),
    NotFound,
}

// Converts our custom AppError into a user-friendly HTTP response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::PoolError(e) => {
                tracing::error!("Pool error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            AppError::RedisError(e) => {
                tracing::error!("Redis error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Hub not found".to_string()),
            AppError::IoError(e) => {
                tracing::error!("IO error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            AppError::S3PutError(e) => {
                tracing::error!("S3 Put Object error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            AppError::S3GetError(e) => {
                tracing::error!("S3 Get Object error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
            AppError::UploadError(e) => {
                tracing::error!("Upload error: {:?}", e);
                (StatusCode::BAD_REQUEST, "File upload failed".to_string())
            }
            AppError::ZipError(e) => {
                tracing::error!("Zip creation error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                )
            }
        };
        (status, error_message).into_response()
    }
}

// `From` trait implementations to allow using the `?` operator on different error types.
impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::PoolError(err)
    }
}
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::RedisError(err)
    }
}
impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>>
    for AppError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>,
    ) -> Self {
        AppError::S3PutError(err)
    }
}
impl From<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>>
    for AppError
{
    fn from(
        err: aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>,
    ) -> Self {
        AppError::S3GetError(err)
    }
}
impl From<zip::result::ZipError> for AppError {
    fn from(err: zip::result::ZipError) -> Self {
        AppError::ZipError(err)
    }
}
impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::IoError(err)
    }
}

// Data model for a Hub, stored as JSON in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hub {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<FileInfo>,
    pub whiteboard: Vec<PathData>,
}

// Data model for file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub filename: String,
    pub size: u64,
}

// The response structure for the create_hub handler.
#[derive(Serialize)]
pub struct CreateHubResponse {
    id: String,
    url: String,
    text_url: String,
    expires_at: String,
}

/// Handler to create a new hub.
pub async fn create_hub(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<CreateHubResponse>), AppError> {
    let mut conn = state.redis.get().await?;

    let id = nanoid!(10);
    let now = Utc::now();

    let hub = Hub {
        id: id.clone(),
        content: String::from("Welcome to your ephemeral hub!"),
        created_at: now,
        files: Vec::new(),
        whiteboard: Vec::new(),
    };

    let hub_json = serde_json::to_string(&hub).unwrap();
    let ttl_seconds = Duration::hours(24).num_seconds() as usize;

    redis::cmd("SET")
        .arg(format!("hub:{}", id))
        .arg(hub_json)
        .arg("EX")
        .arg(ttl_seconds)
        .query_async::<()>(&mut *conn)
        .await?;

    debug!("Created new hub with id: {}", id);
    let base_url = "http://localhost:3000";
    let expires_at = now + Duration::hours(24);

    Ok((
        StatusCode::CREATED,
        Json(CreateHubResponse {
            id: id.clone(),
            url: format!("{}/api/hubs/{}", base_url, id),
            text_url: format!("{}/api/hubs/{}/text", base_url, id),
            expires_at: expires_at.to_rfc3339(),
        }),
    ))
}

/// Handler to get the content of a hub.
pub async fn get_hub(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Hub>, AppError> {
    let mut conn = state.redis.get().await?;

    let key = format!("hub:{}", id);
    let hub_json: Option<String> = conn.get(key).await?;

    match hub_json {
        Some(json) => {
            let hub: Hub = serde_json::from_str(&json).unwrap();
            Ok(Json(hub))
        }
        None => Err(AppError::NotFound),
    }
}

/// Handler to update the text bin for a hub.
pub async fn update_text_bin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: String,
) -> Result<StatusCode, AppError> {
    let mut conn = state.redis.get().await?;

    let key = format!("hub:{}", id);
    let hub_json: Option<String> = conn.get(&key).await?;

    if let Some(json) = hub_json {
        let mut hub: Hub = serde_json::from_str(&json).unwrap();
        hub.content = body;

        let updated_json = serde_json::to_string(&hub).unwrap();
        let ttl: isize = conn.ttl(&key).await?;

        if ttl > 0 {
            redis::cmd("SET")
                .arg(&key)
                .arg(updated_json)
                .arg("EX")
                .arg(ttl)
                .query_async::<()>(&mut *conn)
                .await?;
        }

        debug!("Updated text for hub id: {}", id);
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

/// Handler to upload one or more files to a hub.
pub async fn upload_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<StatusCode, AppError> {
    let mut conn = state.redis.get().await?;
    let key = format!("hub:{}", id);

    // Get the hub metadata from Redis first.
    let hub_json: Option<String> = conn.get(&key).await?;
    let mut hub: Hub = serde_json::from_str(&hub_json.ok_or(AppError::NotFound)?).unwrap();

    // Iterate over each part of the multipart upload.
    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap_or("unknown_file").to_string();
        let s3_key = format!("{}/{}", id, filename);

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::UploadError(e.to_string()))?;
        let file_size = data.len() as u64;

        // Stream the file content to the S3 bucket.
        let body = ByteStream::from(data);

        let bucket = env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set");
        state
            .s3
            .put_object()
            .bucket(&bucket)
            .key(&s3_key)
            .body(body)
            .send()
            .await?;

        // Update the hub metadata with the new file info.
        hub.files.push(FileInfo {
            filename,
            size: file_size,
        });
    }

    // Save the updated hub metadata back to Redis, preserving the TTL.
    let updated_json = serde_json::to_string(&hub).unwrap();
    let ttl: isize = conn.ttl(&key).await?;
    if ttl > 0 {
        redis::cmd("SET")
            .arg(&key)
            .arg(updated_json)
            .arg("EX")
            .arg(ttl)
            .query_async::<()>(&mut *conn)
            .await?;
    }

    Ok(StatusCode::OK)
}

/// Handler to download all content of a hub as a single zip archive.
pub async fn download_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let mut conn = state.redis.get().await?;
    let key = format!("hub:{}", id);

    // Get the hub metadata from Redis.
    let hub_json: Option<String> = conn.get(&key).await?;
    let hub: Hub = serde_json::from_str(&hub_json.ok_or(AppError::NotFound)?).unwrap();

    // Create a zip archive in an in-memory buffer.
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip: ZipWriter<Cursor<&mut Vec<u8>>> = ZipWriter::new(cursor);

    // Add the text bin content to the zip.
    zip.start_file("ephemeral_text_bin.txt", FileOptions::<()>::default())?;
    zip.write_all(hub.content.as_bytes())?;

    // Fetch each file from S3 and add it to the zip.
    for file_info in hub.files {
        let s3_key = format!("{}/{}", id, file_info.filename);
        let bucket = env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set");
        let object = state
            .s3
            .get_object()
            .bucket(&bucket)
            .key(&s3_key)
            .send()
            .await?;
        let data = object.body.collect().await.unwrap().into_bytes();

        zip.start_file(&file_info.filename, FileOptions::<()>::default())?;
        zip.write_all(&data)?;
    }

    zip.finish()?;

    // Manually build the HTTP response with the correct headers and body.
    let body = Body::from(buffer);
    let filename = format!("ephemeral_hub_{}.zip", id);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(body)
        .unwrap();

    Ok(response)
}

pub async fn verify_bucket_access(
    s3_client: &S3Client,
    bucket_name: &str,
) -> Result<(), SdkError<HeadBucketError>> {
    s3_client.head_bucket().bucket(bucket_name).send().await?;
    Ok(())
}
