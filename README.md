<p align="center">
  <img src="ephemeral_web/assets/logo.png" alt="ephemeral hub" width="200">
</p>

<h1 align="center">
  Ephemeral Hub
</h1>

[![Netlify Status](https://api.netlify.com/api/v1/badges/ed0b5f52-0792-4dc4-b33b-780bc3d1f1a1/deploy-status)](https://app.netlify.com/projects/ephemeral-hub/deploys)

**Ephemeral Hub** is a temporary, no-login-required hub for text, files, and collaborative brainstorming. It provides a digital equivalent of a scrap piece of paper or a temporary whiteboard, where all content is automatically deleted after a set time.

## This project is a monorepo containing

- ephemeral_backend: An Axum-based Rust server that manages spaces, files, and WebSocket connections.
- ephemeral_web: A Dioxus-based frontend application compiled to WebAssembly.
- ephemeral_cli: A command-line interface for power users to interact with spaces.

### 🚀 Getting Started

These instructions will get a copy of the project up and running on your local machine for development and testing purposes.

**Prerequisites**: You will need the following tools installed on your system:

```bash
Rust & Cargo: https://www.rust-lang.org/tools/install
Docker & Docker Compose: https://www.docker.com/products/docker-desktop/
Dioxus CLI: cargo install dioxus-cli
```

#### 💻 Running Locally

Running the entire application stack is managed with two commands in two separate terminals. All commands should be run from the root ephemeral/ directory.

#### 🖥️ Terminal 1: Start the Backend Stack

```bash
# This single command will build the backend's Docker image and start the backend server, a Redis database, and a MinIO S3-compatible file store.

docker-compose up --build

# The backend API will be available at http://127.0.0.1:3000.
# The MinIO web console will be available at http://127.0.0.1:9001 (user: minioadmin, pass: minioadmin).
```

### 🌐 Terminal 2: Start the Frontend

```bash
# This command will build and serve the Dioxus web application with hot-reloading.

dx serve --package ephemeral_web --open

# The frontend will be available at the URL provided by the CLI (usually http://127.0.0.1:8080).
```
