# Getting Started with RustyGPT

This guide will help you set up and run RustyGPT locally for development.

## Prerequisites

Before you begin, ensure you have the following installed:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) (comes with Rust)
- [Docker](https://docs.docker.com/get-docker/) and [Docker Compose](https://docs.docker.com/compose/install/) (for local development)
- [PostgreSQL](https://www.postgresql.org/download/) (if not using Docker)
- [Git](https://git-scm.com/downloads)

## Installation

1. **Clone the Repository**

   ```bash
   git clone https://github.com/VannaDii/RustyGPT.git
   cd RustyGPT
   ```

2. **Set Up Environment Variables**

   Copy the environment template and configure it with your settings:

   ```bash
   cp .env.template .env
   ```

   Edit the `.env` file with your preferred text editor and set the following variables:

   ```
   # Database connection
   DATABASE_URL=postgres://postgres:postgres@localhost/rusty_gpt

   # OAuth credentials (if you want to use authentication)
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

   > **Note**: For local development, you can leave the OAuth credentials as placeholders if you don't need authentication.

## Running the Application

You can run RustyGPT using Docker Compose or by running the backend and frontend separately.

### Option 1: Using Docker Compose (Recommended)

This option sets up the entire application stack, including PostgreSQL:

```bash
docker-compose up --build
```

The application will be available at:

- Frontend: http://localhost:3000
- Backend API: http://localhost:8080

### Option 2: Running Components Separately

#### 1. Set Up the Database

If you're not using Docker, you'll need to set up PostgreSQL manually:

```bash
# Create the database
createdb rusty_gpt

# You may need to run database migrations (if implemented)
# See project documentation for specific instructions
```

#### 2. Run the Backend

```bash
cd backend
cargo run
```

The backend server will start on http://localhost:8080

#### 3. Run the Frontend

In a new terminal:

```bash
cd frontend
trunk serve
```

The frontend development server will start on http://localhost:8000

## Development Workflow

### Building the Project

To build the entire project:

```bash
cargo build
```

To build for production:

```bash
cargo build --release
```

### Running Tests

To run all tests:

```bash
cargo test
```

To run tests for a specific crate:

```bash
cargo test -p backend  # or frontend, shared
```

### Code Formatting and Linting

Format your code using rustfmt:

```bash
cargo fmt
```

Run clippy to catch common mistakes:

```bash
cargo clippy
```

## Project Structure

- `backend/`: Axum server providing authentication and conversation APIs
- `frontend/`: Yew-based web frontend
- `shared/`: Common models and utilities used by both frontend and backend

## Next Steps

- Explore the [Architecture Documentation](architecture.md) to understand the project structure
- Check out the [API Reference](api/index.html) for detailed API documentation
- Read the [Contributing Guidelines](https://github.com/VannaDii/RustyGPT/blob/main/CONTRIBUTING.md) if you want to contribute

## Troubleshooting

### Common Issues

1. **Database Connection Errors**

   Ensure PostgreSQL is running and the `DATABASE_URL` in your `.env` file is correct.

2. **Build Errors**

   Make sure you have the latest stable Rust version:

   ```bash
   rustup update stable
   ```

3. **Frontend Not Loading**

   Ensure you have the WebAssembly target installed:

   ```bash
   rustup target add wasm32-unknown-unknown
   ```

4. **OAuth Authentication Issues**

   For local development, you may need to register OAuth applications with GitHub or Apple and update your `.env` file with the correct credentials.

### Getting Help

If you encounter any issues not covered here:

- Check the [GitHub Issues](https://github.com/VannaDii/RustyGPT/issues) to see if your problem has been reported
- Open a new issue if needed, providing detailed information about your environment and the problem
