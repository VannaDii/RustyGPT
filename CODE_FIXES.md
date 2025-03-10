# Code Fixes and Development Notes

## SQLx Configuration

This project uses [SQLx](https://github.com/launchbadge/sqlx) for database access, which requires special handling in development and CI environments.

### Local Development

For local development, you have two options:

1. **Online Mode (Default)**: SQLx will connect to the database at compile time to check your SQL queries.

   - Ensure your database is running and accessible
   - Set the `DATABASE_URL` environment variable in your `.env` file
   - Example: `DATABASE_URL=postgres://postgres:postgres@localhost/rusty_gpt`

2. **Offline Mode**: Use a prepared query cache without a database connection at compile time.
   - First, prepare the query cache: `cargo sqlx prepare`
   - This creates a `.sqlx` directory with the prepared queries
   - Set the `SQLX_OFFLINE=true` environment variable when building
   - Example: `SQLX_OFFLINE=true cargo build`

### CI Environment

In the GitHub Actions workflows (CI and Lint), we use a combination of both approaches:

1. We set up a PostgreSQL service in the CI environment
2. We create the `.env` file with the `DATABASE_URL`
3. We initialize the database with schema files from `.data/postgres-init`
4. We install the SQLx CLI and prepare the query cache
5. We build and test the project in offline mode using the prepared query cache

This approach ensures that:

- SQL queries are validated against the actual database schema
- The build process doesn't require a database connection once the query cache is prepared
- CI builds are more reliable and don't depend on database connectivity during the build step

### Troubleshooting

If you encounter errors like:

```
error: set `DATABASE_URL` to use query macros online, or run `cargo sqlx prepare` to update the query cache
```

You need to either:

1. Set the `DATABASE_URL` environment variable and ensure your database is running, or
2. Run `cargo sqlx prepare` to update the query cache and then build with `SQLX_OFFLINE=true`

### Committing the Query Cache

It's recommended to commit the `.sqlx` directory to your repository. This allows other developers and CI systems to build the project without needing a database connection, as long as they use `SQLX_OFFLINE=true`.
