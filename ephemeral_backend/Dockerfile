# ---- Builder Stage ----
# Use the official Rust image as a build environment.
FROM rust:1.79 AS builder

# Set a working directory for the entire application.
WORKDIR /usr/src/app

# Install build dependencies that are commonly needed.
RUN apt-get update && apt-get install -y libssl-dev pkg-config

# Copy the entire workspace context into the container.
# This is the simplest way to handle workspace builds.
COPY . .

# Build only the specific `ephemeral_backend` package in release mode.
RUN cargo build --release --package ephemeral_backend

# ---- Runner Stage ----
# Use a minimal base image for the final container.
FROM debian:bookworm-slim AS runner

# Install runtime dependencies.
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy the compiled binary from the correct path in the builder stage.
COPY --from=builder /usr/src/app/target/release/ephemeral_backend .

# Expose the port the server will run on.
EXPOSE 3000

# Set the command to run the application.
CMD ["./ephemeral_backend"]
