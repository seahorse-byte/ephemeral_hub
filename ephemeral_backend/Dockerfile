# ---- Builder Stage ----
# Use the official Rust image as a build environment.
# Using a specific version ensures reproducible builds.
FROM rust:1.78 AS builder

# Set the working directory inside the container.
WORKDIR /usr/src/ephemeral_backend

# Install build dependencies (like OpenSSL) that might be needed for some crates.
# This is a common requirement for Rust networking libraries.
RUN apt-get update && apt-get install -y libssl-dev pkg-config

# Copy the Cargo configuration files first.
# This allows Docker to cache the dependencies layer if they haven't changed.
COPY ./Cargo.toml ./Cargo.lock* ./

# Create a dummy main.rs to build only the dependencies.
# This is a caching trick: if only your app code changes, Docker won't
# need to re-download and re-compile all your dependencies.
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/ephemeral_backend*

# Now, copy the actual source code into the container.
COPY . .

# Build the application for release.
RUN cargo build --release

# ---- Runner Stage ----
# Use a minimal base image for the final container.
# "debian:bookworm-slim" is small and secure.
FROM debian:bookworm-slim AS runner

# Install runtime dependencies. OpenSSL is needed for HTTPS.
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy the compiled binary from the builder stage.
COPY --from=builder /usr/src/ephemeral_backend/target/release/ephemeral_backend .

# Expose the port the server will run on.
EXPOSE 3000

# Set the command to run the application.
CMD ["./ephemeral_backend"]