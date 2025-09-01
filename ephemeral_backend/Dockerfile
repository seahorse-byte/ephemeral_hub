# ---- Builder Stage ----
# Use the latest stable Rust image as the build environment.
FROM rust:latest AS builder

# Set the working directory inside the container.
WORKDIR /usr/src/app

# Copy the entire project (the workspace) into the container.
COPY . .

# Build only the specific `ephemeral_backend` package in release mode.
# This is a simpler and more robust build step.
RUN cargo build --release --package ephemeral_backend

# ---- Runner Stage ----
# Use a slim, modern Debian image for the final, small container.
FROM debian:trixie-slim AS runner

# Install necessary certificates for making HTTPS requests.
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a non-root user for better security.
RUN useradd -ms /bin/bash appuser

# Set the working directory for the runner.
WORKDIR /home/appuser/

# Copy the compiled binary from the builder stage.
COPY --from=builder /usr/src/app/target/release/ephemeral_backend .

# Change the ownership of the application files to the new user.
RUN chown -R appuser:appuser .

# Switch to the non-root user.
USER appuser

# Expose the port the application will run on.
EXPOSE 3000

# The command to run when the container starts.
CMD ["./ephemeral_backend"]