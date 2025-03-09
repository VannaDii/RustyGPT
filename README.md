# RustyGPT

## Overview

**RustyGPT** is a learning-driven Rust project focused on mastering Rust for building efficient, scalable backend systems. The project explores **Axum** for API development, **PostgreSQL** for database management, and **AI model integration** in Rust.

## Features

- **RESTful API** using **Axum** for high-performance web services.
- **PostgreSQL integration** with stored procedures for secure, efficient database access.
- **OAuth authentication** via Apple and GitHub (never Google or Meta).
- **AI model integration** using local inference engines.
- **Docker Compose setup** for seamless environment management.
- **Unit-tested architecture** ensuring reliability and maintainability.

## Project Goals

- Achieve proficiency in Rust for backend development.
- Build scalable and maintainable REST APIs.
- Implement secure user authentication and authorization.
- Integrate AI models into server-side applications.
- Maintain a structured, best-practice-driven Rust codebase.

## Tech Stack

- **Programming Language:** Rust
- **Web Framework:** Axum
- **Database:** PostgreSQL
- **Authentication:** Local accounts & OAuth (Apple, GitHub)
- **Containerization:** Docker Compose
- **Testing:** Unit tests for API and database interactions
- **AI Models:** Local inference, no external API dependencies

## Setup & Installation

1. **Clone the Repository**

   ```sh
   git clone https://github.com/VannaDii/RustyGPT.git rusty_gpt
   cd rusty_gpt
   ```

2. **Install Dependencies**

   - Ensure you have Rust installed ([Rustup](https://rustup.rs/)).
   - Install Docker and Docker Compose.
   - Configure environment variables (see `.env.example`).

3. **Run the Project**

   ```sh
   docker-compose up --build
   ```

4. **Run Tests**
   ```sh
   cargo test
   ```

## API Documentation

- OpenAPI Spec available at `/swagger` when the server is running.
- Endpoints follow REST best practices.

## Contributing

This is an evolving project focused on learning and best practices. Contributions and suggestions are welcome!

## License

MIT License. See `LICENSE` for details.
