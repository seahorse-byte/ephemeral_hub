# Run Backend + Frontend

## Prerequisites

Make sure you have Docker installed and running.
Open two separate terminal windows or tabs.
All commands should be run from the parent ephemeral directory.

### 🖥️ Terminal 1: Start the Backend

```bash
# Start Redis
# This command starts the Redis database in a Docker container.

docker run --name ephemeral-redis -p 6379:6379 -d redis
# (If it's already running, you can use docker start ephemeral-redis)
```

```bash
# Start MinIO
# This command starts the S3-compatible file storage in a Docker container.

docker run \
  -p 9000:9000 \
  -p 9001:9001 \
  --name ephemeral-minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  -d minio/minio server /data --console-address ":9001"
# (If it's already running, you can use docker start ephemeral-minio)
```

```bash
# Run the Backend Server
# This command runs your Axum server and provides it with the necessary environment variables to connect to Redis and MinIO.

REDIS_URL=redis://127.0.0.1/ \
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
cargo run --package ephemeral_backend
```

### 🌐 Terminal 2: Start the Frontend

```bash
# Serve the Dioxus App
# This command tells the Dioxus CLI to build and serve the ephemeral_web package.
dx serve --package ephemeral_web
# After running these commands, your backend will be live on http://127.0.0.1:3000 and your frontend will be available at the URL provided by dx (usually http://127.0.0.1:8080).
```
