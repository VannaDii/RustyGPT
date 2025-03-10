# RustyGPT Documentation

This directory contains the documentation for the RustyGPT project, which is hosted on GitHub Pages.

## Structure

- `index.html`: Main landing page for the documentation site
- `api/`: Generated API documentation for the Rust crates
  - `backend/`: API documentation for the backend crate
  - `frontend/`: API documentation for the frontend crate
  - `shared/`: API documentation for the shared crate

## Development

The documentation is automatically generated and deployed to GitHub Pages using the GitHub Actions workflow defined in `.github/workflows/docs.yml`. This workflow:

1. Generates Rust API documentation using `cargo doc`
2. Copies the generated documentation to the `docs/api/` directory
3. Deploys the contents of the `docs/` directory to GitHub Pages

## Local Development

To build and view the documentation locally:

1. Generate the Rust API documentation:

   ```bash
   cargo doc --no-deps --workspace
   ```

2. Copy the generated documentation to the `docs/api/` directory:

   ```bash
   mkdir -p docs/api
   cp -r target/doc/* docs/api/
   ```

3. Serve the documentation locally using a simple HTTP server:

   ```bash
   cd docs
   python -m http.server 8000
   ```

4. Open your browser and navigate to `http://localhost:8000`

## Contributing

If you'd like to contribute to the documentation:

1. For changes to the Rust API documentation, update the doc comments in the source code
2. For changes to the documentation site, modify the files in the `docs/` directory
3. Submit a pull request with your changes

Please see the [Contributing Guidelines](https://github.com/VannaDii/RustyGPT/blob/main/CONTRIBUTING.md) for more information.
