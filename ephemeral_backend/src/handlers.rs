use crate::AppState;
use aws_sdk_s3::primitives::ByteStream;
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
use std::io::{self, Cursor, Write};
use tracing::debug;
use zip::write::{FileOptions, ZipWriter};

// The AppError enum, updated to handle all necessary error types.
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
            AppError::NotFound => (StatusCode::NOT_FOUND, "Space not found".to_string()),
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

// Data model for a Space, stored as JSON in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Space {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub files: Vec<FileInfo>,
}

// Data model for file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub filename: String,
    pub size: u64,
}

// The response structure for the create_space handler.
#[derive(Serialize)]
pub struct CreateSpaceResponse {
    id: String,
    url: String,
    text_url: String,
    expires_at: String,
}

/// Handler to create a new space.
pub async fn create_space(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<CreateSpaceResponse>), AppError> {
    let mut conn = state.redis.get().await?;

    let id = nanoid!(10);
    let now = Utc::now();
    let space = Space {
        id: id.clone(),
        content: String::from("Welcome to your ephemeral space!"),
        created_at: now,
        files: Vec::new(),
    };

    let space_json = serde_json::to_string(&space).unwrap();
    let ttl_seconds = Duration::hours(24).num_seconds() as usize;

    redis::cmd("SET")
        .arg(format!("space:{}", id))
        .arg(space_json)
        .arg("EX")
        .arg(ttl_seconds)
        .query_async::<()>(&mut *conn)
        .await?;

    debug!("Created new space with id: {}", id);

    let base_url = "http://127.0.0.1:3000";
    let expires_at = now + Duration::hours(24);

    Ok((
        StatusCode::CREATED,
        Json(CreateSpaceResponse {
            id: id.clone(),
            url: format!("{}/api/spaces/{}", base_url, id),
            text_url: format!("{}/api/spaces/{}/text", base_url, id),
            expires_at: expires_at.to_rfc3339(),
        }),
    ))
}

/// Handler to get the content of a space.
pub async fn get_space(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Space>, AppError> {
    let mut conn = state.redis.get().await?;

    let key = format!("space:{}", id);
    let space_json: Option<String> = conn.get(key).await?;

    match space_json {
        Some(json) => {
            let space: Space = serde_json::from_str(&json).unwrap();
            Ok(Json(space))
        }
        None => Err(AppError::NotFound),
    }
}

/// Handler to update the text bin for a space.
pub async fn update_text_bin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    body: String,
) -> Result<StatusCode, AppError> {
    let mut conn = state.redis.get().await?;

    let key = format!("space:{}", id);
    let space_json: Option<String> = conn.get(&key).await?;

    if let Some(json) = space_json {
        let mut space: Space = serde_json::from_str(&json).unwrap();
        space.content = body;

        let updated_json = serde_json::to_string(&space).unwrap();
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

        debug!("Updated text for space id: {}", id);
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound)
    }
}

/// Handler to upload one or more files to a space.
pub async fn upload_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Result<StatusCode, AppError> {
    let mut conn = state.redis.get().await?;
    let key = format!("space:{}", id);

    // Get the space metadata from Redis first.
    let space_json: Option<String> = conn.get(&key).await?;
    let mut space: Space = serde_json::from_str(&space_json.ok_or(AppError::NotFound)?).unwrap();

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
        state
            .s3
            .put_object()
            .bucket("ephemeral")
            .key(&s3_key)
            .body(body)
            .send()
            .await?;

        // Update the space metadata with the new file info.
        space.files.push(FileInfo {
            filename,
            size: file_size,
        });
    }

    // Save the updated space metadata back to Redis, preserving the TTL.
    let updated_json = serde_json::to_string(&space).unwrap();
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

/// Handler to download all content of a space as a single zip archive.
pub async fn download_files(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let mut conn = state.redis.get().await?;
    let key = format!("space:{}", id);

    // Get the space metadata from Redis.
    let space_json: Option<String> = conn.get(&key).await?;
    let space: Space = serde_json::from_str(&space_json.ok_or(AppError::NotFound)?).unwrap();

    // Create a zip archive in an in-memory buffer.
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip: ZipWriter<Cursor<&mut Vec<u8>>> = ZipWriter::new(cursor);

    // Add the text bin content to the zip.
    zip.start_file("ephemeral_text_bin.txt", FileOptions::<()>::default())?;
    zip.write_all(space.content.as_bytes())?;

    // Fetch each file from S3 and add it to the zip.
    for file_info in space.files {
        let s3_key = format!("{}/{}", id, file_info.filename);
        let object = state
            .s3
            .get_object()
            .bucket("ephemeral")
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
    let filename = format!("ephemeral_space_{}.zip", id);

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
