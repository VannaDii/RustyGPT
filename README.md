# RustyGPT

[![CI](https://github.com/VannaDii/RustyGPT/actions/workflows/ci.yml/badge.svg)](https://github.com/VannaDii/RustyGPT/actions/workflows/ci.yml) [![Lint](https://github.com/VannaDii/RustyGPT/actions/workflows/lint.yml/badge.svg)](https://github.com/VannaDii/RustyGPT/actions/workflows/lint.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/VannaDii/RustyGPT/blob/main/CONTRIBUTING.md) [![Rust Version](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org/)

## Overview

**RustyGPT** is a learning-driven Rust project focused on mastering Rust for building efficient, scalable backend systems with a modern frontend. The project explores **Axum** for API development, **PostgreSQL** for database management, and **AI model integration** in Rust.

## Project Structure

The project follows a clean architecture with clear separation of concerns:

- **Backend**: Rust-based Axum server providing authentication and conversation APIs
  - **Handlers**: Handle HTTP requests and responses
  - **Routes**: Define API endpoints and group related routes
  - **Services**: Implement business logic
- **Frontend**: Rust-based web frontend using Yew
  - **Components**: Reusable UI elements following atomic design principles
  - **State Management**: Yewdux for global state with optimized performance
  - **Layout**: DaisyUI admin dashboard with responsive design
  - **Accessibility**: WCAG 2.2 AA compliant components
- **Shared**: Common models and utilities used by both frontend and backend
  - **Models**: Data structures for conversations, messages, users, and streaming functionality

## Features

- **RESTful API** using **Axum** for high-performance web services
- **Modern Admin Dashboard** using DaisyUI and Yew with responsive design
- **Component-driven UI** following atomic design principles
- **Optimized WebAssembly** with sub-1MB initial payload
- **Real-time streaming** with Server-Sent Events (SSE) for message delivery
- **Interactive interface** with real-time updates
- **PostgreSQL integration** with stored procedures for secure, efficient database access
- **OAuth authentication** via GitHub (with Apple support planned)
- **AI model integration** using local inference engines
- **Docker Compose setup** for seamless environment management
- **Unit-tested architecture** ensuring reliability and maintainability
- **Accessibility compliance** with WCAG 2.2 AA standards
- **Configuration-based URLs** for flexible deployment across environments
- **Built-in observability** with Prometheus metrics, JSON/text logging, and health probes
- **Durable SSE streams** with optional persistent cursors and backpressure metrics

## Chat & Streaming Functionality

The application features a real-time chat system with streaming message delivery:

1. **Message Sending**: Users can send messages to conversations via the API
2. **Server-Sent Events**: The backend uses SSE to stream message chunks to connected clients
3. **Real-time Updates**: The frontend receives and displays message chunks as they arrive
4. **Streaming UI**: Messages are displayed with typing indicators while streaming
5. **Conversation Management**: Users can view and interact with multiple conversations

## CLI Interface

The RustyGPT CLI provides a command-line interface for interacting with AI models locally:

### Chat Command

Start an interactive chat session with a local AI model:

```sh
# Basic chat with default settings
cargo run --bin cli chat

# Chat with custom model
cargo run --bin cli chat --model /path/to/your/model.gguf

# Chat with custom parameters
cargo run --bin cli chat \
  --model /path/to/model.gguf \
  --temperature 0.7 \
  --max-tokens 512 \
  --system "You are a helpful coding assistant."
```

### Available Options

- `--model, -m`: Path to the GGUF model file (auto-detects hardware optimization)
- `--temperature, -t`: Response creativity (0.0-1.0, lower = more focused)
- `--max-tokens`: Maximum tokens per response (model-dependent default)
- `--system, -s`: System message to set AI behavior

### Features

- **Hardware Optimization**: Automatically detects CPU/GPU capabilities and optimizes model parameters
- **Interactive Session**: Type messages and receive AI responses in real-time
- **Exit Commands**: Type `exit`, `quit`, or `q` to end the session
- **Token Usage**: Shows token statistics for each response
- **Error Handling**: Graceful handling of model loading and generation errors

### Other CLI Commands

```sh
# Generate OpenAPI specification
cargo run --bin cli spec

# Generate shell completions
cargo run --bin cli completion bash

# Generate configuration file
cargo run --bin cli config

# Start the backend server
cargo run --bin cli serve --port 8080
```

## Observability

RustyGPT exposes production-ready telemetry out of the box:

| Signal            | Endpoint / Location           | Notes                                                                              |
| ----------------- | ----------------------------- | ---------------------------------------------------------------------------------- |
| **Metrics**       | `GET /metrics`                | Prometheus exposition including HTTP, SSE, DB bootstrap, and health counters/gauges |
| **Liveness**      | `GET /healthz`                | Returns `200` when the API process is running                                      |
| **Readiness**     | `GET /readyz`                 | Verifies database connectivity and stored procedure availability                   |
| **Structured logs** | stdout / stderr             | `logging.format = "json"` enables machine-readable JSON logs for aggregation       |

### Metrics Quick Start

Add the following scrape config to your Prometheus deployment:

```yaml
scrape_configs:
  - job_name: rustygpt
    metrics_path: /metrics
    static_configs:
      - targets:
          - rustygpt.example.com:8080
```

Key metric families include:

- `http_requests_total` / `http_request_duration_seconds` for inbound API traffic
- `sse_events_sent_total`, `sse_events_dropped_total`, and `sse_queue_depth` for streaming health
- `db_bootstrap_*` and `db_{liveness,readiness}_checks_total` covering database lifecycle

### Structured Logging

Configure JSON logs in `config.toml`:

```toml
[logging]
level = "info"
format = "json"
```

With this enabled, each log line includes request identifiers, matched routes, status codes, and latencies to simplify ingestion into log pipelines such as Loki, CloudWatch, or ELK.

## SSE Durability & Backpressure

The SSE coordinator supports optional on-disk persistence and queue instrumentation:

- Enable durable cursors by setting `sse.persistence.enabled = true`; the coordinator stores events in PostgreSQL via stored procedures.
- Configure SSE retention with `sse.persistence.retention_hours` (clamped to 24–72h) and tune replay bounds with `sse.persistence.max_events_per_user` / `sse.persistence.prune_batch_size`.
- Control congestion handling with `sse.backpressure.drop_strategy` (`drop_tokens` or `drop_tokens_and_system`) and monitor queue pressure through `sse_queue_depth`/`sse_queue_occupancy_ratio`.

These knobs allow operators to tune RustyGPT for multi-tenant workloads while maintaining delivery guarantees for critical events.

## Observability

- Export Prometheus metrics via `/metrics`; new counters for auth, rate limits, and SSE replay are documented in `docs/operations/auth.md` and `docs/operations/rate_limits.md`.
- Import the dashboards in `deploy/grafana/*.json` using Grafana's **Import Dashboard** workflow and point them at the RustyGPT Prometheus data source. The JSON files are kept as single-source-of-truth for CI and can be versioned alongside infrastructure changes.
- Cross-check metric availability with `promtool` or by querying Prometheus (e.g., `sum by (result) (rustygpt_auth_logins_total)`); the dashboards expect the metric names emitted by the latest server build.

## CLI Authentication Workflow

The CLI stores session cookies locally to reuse browser-compatible auth flows:

1. Run `rustygpt session login` and enter email/password when prompted. Cookies are persisted to the path displayed after login.
2. Inspect the active session with `rustygpt session me`, which calls `/api/auth/me` and prints the stored profile and expiry timestamps.
3. Revoke local credentials with `rustygpt session logout`; the command clears the cookie jar and notifies the backend.

Most interactive subcommands (e.g., `chat`, `reply`) now reuse the shared cookie jar and automatically attach CSRF headers when posting data.

## Authentication Flow

The application supports OAuth authentication with both GitHub and Apple:

1. **Initiate OAuth**: User visits `/api/oauth/github` or `/api/oauth/apple` to start the authentication flow
2. **Provider Redirect**: User is redirected to the OAuth provider (GitHub or Apple)
3. **Callback**: Provider redirects back to our callback endpoint with an authorization code
4. **Token Exchange**: Backend exchanges the code for an access token
5. **User Creation/Login**: Backend creates or retrieves a user account based on the OAuth provider ID
6. **Success Page**: User is redirected to the success page with their user ID

## API Endpoints

### Authentication

- `GET /api/oauth/github`: Initiate GitHub OAuth flow
- `GET /api/oauth/github/callback`: Handle GitHub OAuth callback
- `POST /api/oauth/github/manual`: Manual GitHub OAuth (for testing)
- `GET /api/oauth/apple`: Initiate Apple OAuth flow
- `GET /api/oauth/apple/callback`: Handle Apple OAuth callback
- `POST /api/oauth/apple/manual`: Manual Apple OAuth (for testing)

### Protected Routes

- `GET /api/conversations`: Get all conversations for the authenticated user
- `POST /api/conversations/{conversation_id}/messages`: Send a message to a conversation
- `GET /api/stream/{user_id}`: Connect to the SSE stream for real-time message updates

## Tech Stack

- **Programming Language:** Rust
- **Backend Framework:** Axum
- **Frontend Framework:** Yew
- **UI Components:** DaisyUI and Tailwind CSS
- **State Management:** Yewdux
- **Data Visualization:** plotters-canvas
- **Real-time Communication:** Server-Sent Events (SSE)
- **Database:** PostgreSQL
- **Authentication:** OAuth (GitHub, future Apple support)
- **Containerization:** Docker Compose
- **Testing:** Unit tests for both backend and frontend
- **Bundler:** Trunk with wasm-opt
- **AI Models:** Local inference, no external API dependencies

## Environment Variables

The application requires the following environment variables:

```
# Database connection
DATABASE_URL=postgres://postgres:postgres@localhost/rusty_gpt

# OAuth credentials
GITHUB_CLIENT_ID=your_github_client_id
GITHUB_CLIENT_SECRET=your_github_client_secret
GITHUB_REDIRECT_URI=http://localhost:8080/api/oauth/github/callback
GITHUB_AUTH_URL=https://github.com/login/oauth/authorize
GITHUB_TOKEN_URL=https://github.com/login/oauth/access_token

APPLE_CLIENT_ID=your_apple_client_id
APPLE_REDIRECT_URI=http://localhost:8080/api/oauth/apple/callback
APPLE_AUTH_URL=https://appleid.apple.com/auth/authorize
APPLE_TOKEN_URL=https://appleid.apple.com/auth/token
```

## Setup & Installation

1. **Clone the Repository**

   ```sh
   git clone https://github.com/VannaDii/RustyGPT.git rusty_gpt
   cd rusty_gpt
   ```

2. **Install Dependencies**

   - Ensure you have [Rust installed](https://rustup.rs)
   - Ensure you have [Just installed](https://just.systems)
   - Install [Docker and Docker Compose](https://docs.docker.com/compose/install/)
   - Configure environment variables (create a `.env` file based on `.env.template`)
     - `cp .env.template .env`

3. **Install Tools**

   ```sh
   just install
   ```

4. **Run with Live Reloading**

   ```sh
   just dev
   ```

5. **Run with Docker Compose**

   ```sh
   docker-compose up --build
   ```

6. **Run Backend Only**

   ```sh
   cd backend
   cargo run
   ```

   The server will start on `http://localhost:8080`

7. **Run Frontend Only**

   ```sh
   cd frontend
   trunk serve
   ```

8. **Run Tests**
   ```sh
   just test
   ```

## Contributing

We welcome contributions from the community! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details on how to get started, coding standards, and our development process.

Before contributing, please review our:

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Security Policy](SECURITY.md)
- [Roadmap](ROADMAP.md) for planned features

### Getting Started as a Contributor

1. Fork the repository
2. Create a new branch for your feature or bugfix
3. Make your changes
4. Run tests to ensure they pass
5. Submit a pull request

## Documentation

The project documentation is available on our [GitHub Pages site](https://vannadii.github.io/RustyGPT/).

We provide detailed documentation for the frontend rewrite:

- [Frontend Architecture](docs/frontend-architecture.md) - Overview of the frontend architecture and implementation plan
- [Component Guidelines](docs/component-guidelines.md) - Standards for component development
- [State Management](docs/state-management.md) - Details of the Yewdux implementation
- [Frontend Development Guide](docs/frontend-development.md) - Guide for developers working on the frontend

## Community

- **Issues**: Use GitHub issues to report bugs or request features
- **Discussions**: Join our GitHub discussions for questions and ideas
- **Pull Requests**: Submit PRs for bug fixes or features aligned with our roadmap

## Roadmap

See our [Roadmap](ROADMAP.md) for planned features and improvements, including:

- Workflow and task management
- Image generation capabilities
- And more!

## Changelog

See the [Changelog](CHANGELOG.md) for a history of changes and releases.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
