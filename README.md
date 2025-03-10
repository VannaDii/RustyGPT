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
  - **Components**: Reusable UI elements including chat input, chat list, chat view, and streaming message handling
- **Shared**: Common models and utilities used by both frontend and backend
  - **Models**: Data structures for conversations, messages, users, and streaming functionality

## Features

- **RESTful API** using **Axum** for high-performance web services
- **Real-time streaming** with Server-Sent Events (SSE) for message delivery
- **Interactive chat interface** with real-time message updates
- **PostgreSQL integration** with stored procedures for secure, efficient database access
- **OAuth authentication** via Apple and GitHub
- **AI model integration** using local inference engines
- **Docker Compose setup** for seamless environment management
- **Unit-tested architecture** ensuring reliability and maintainability
- **Configuration-based URLs** for flexible deployment across environments

## Chat & Streaming Functionality

The application features a real-time chat system with streaming message delivery:

1. **Message Sending**: Users can send messages to conversations via the API
2. **Server-Sent Events**: The backend uses SSE to stream message chunks to connected clients
3. **Real-time Updates**: The frontend receives and displays message chunks as they arrive
4. **Streaming UI**: Messages are displayed with typing indicators while streaming
5. **Conversation Management**: Users can view and interact with multiple conversations

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
- **Real-time Communication:** Server-Sent Events (SSE)
- **Database:** PostgreSQL
- **Authentication:** OAuth (Apple, GitHub)
- **Containerization:** Docker Compose
- **Testing:** Unit tests for API and database interactions
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

   - Ensure you have Rust installed ([Rustup](https://rustup.rs/))
   - Install Docker and Docker Compose
   - Configure environment variables (create a `.env` file based on `.env.template`)
     - `cp .env.template .env`

3. **Run with Docker Compose**

   ```sh
   docker-compose up --build
   ```

4. **Run Backend Only**

   ```sh
   cd backend
   cargo run
   ```

   The server will start on `http://localhost:8080`

5. **Run Frontend Only**

   ```sh
   cd frontend
   trunk serve
   ```

6. **Run Tests**
   ```sh
   cargo test
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
