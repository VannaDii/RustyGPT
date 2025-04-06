# Configuration Guide

## Overview

RustyGPT's backend supports a robust configuration system that allows you to customize its behavior using configuration files, environment variables, and static defaults. This guide explains how to use and manage the configuration system.

## Configuration Sources

1. **Configuration File**: JSON or YAML files can be used to define configuration values.
2. **Environment Variables**: All configuration values can be overridden using environment variables prefixed with `RUSTYGPT_`.
3. **Static Defaults**: If no value is provided in the configuration file or environment variables, the system falls back to static defaults.

## Configuration File

### Supported Formats

- YAML (`config.yaml`)
- JSON (`config.json`)

### Default Behavior

- If no `--config` argument is provided, the backend looks for `config.yaml` or `config.json` in the working directory.
- If both files are found, the backend returns an error: `Ambiguous configuration. Config parameter required.`
- If neither file is found, the backend uses environment variables and static defaults.

### Example Configuration

#### YAML

```yaml
server_port: 8080
database_url: 'postgres://localhost:5432/rustygpt'
log_level: 'info'
frontend_path: '../frontend/dist'
```

#### JSON

```json
{
  "server_port": 8080,
  "database_url": "postgres://localhost:5432/rustygpt",
  "log_level": "info",
  "frontend_path": "../frontend/dist"
}
```

## Environment Variables

Environment variables override values in the configuration file. They are case-insensitive and must use the `RUSTYGPT_` prefix.

### Supported Variables

- `RUSTYGPT_SERVER_PORT`: Port number for the server.
- `RUSTYGPT_DATABASE_URL`: Database connection URL.
- `RUSTYGPT_LOG_LEVEL`: Logging level (e.g., `info`, `debug`).
- `RUSTYGPT_FRONTEND_PATH`: Path to the frontend static files directory.

### Example

```bash
export RUSTYGPT_SERVER_PORT=9090
export RUSTYGPT_DATABASE_URL="postgres://user:password@localhost:5432/rustygpt"
export RUSTYGPT_LOG_LEVEL="debug"
export RUSTYGPT_FRONTEND_PATH="/custom/frontend/dist"
```

## Static Defaults

If no value is provided in the configuration file or environment variables, the following defaults are used:

- `server_port`: 8080
- `database_url`: `postgres://localhost:5432/rustygpt`
- `log_level`: `info`
- `frontend_path`: `../frontend/dist`

## Command-Line Arguments

### `--config` or `-c`

Specifies the path to the configuration file.

Example:

```bash
./rustygpt serve --config /path/to/config.yaml
```

## `config` Command

The `config` command generates a configuration file in the specified format.

### Usage

```bash
./rustygpt config --format yaml
```

### Output

- YAML: `config.yaml`
- JSON: `config.json`

If no format is specified, YAML is used by default.

## Validation

All configuration values are validated:

- `server_port` must be between 1 and 65535.
- `database_url` must be a valid URL.
- `log_level` must be a valid logging level.

Errors are logged to `stderr`, and the program exits with a non-zero code.

## Configuration Loading

The configuration is loaded in the following order of precedence:

1. **Command-Line Argument (`--config`)**:

   - If a configuration file path is provided via the `--config` or `-c` argument, the backend attempts to load the file.
   - Supported formats: YAML (`.yaml`) and JSON (`.json`).
   - If the file is invalid or in an unsupported format, an error is displayed.

2. **Command-Line Argument Overrides**:

   - Specific configuration values, such as `--port`, can be provided via command-line arguments.
   - These values override both the configuration file and environment variables.

3. **Configuration File**:

   - If no `--config` argument is provided, the backend looks for `config.yaml` or `config.json` in the working directory.
   - If both files are found, the backend returns an error: `Ambiguous configuration. Config parameter required.`
   - If neither file is found, the backend uses environment variables and static defaults.

4. **Environment Variables**:

   - Any value not provided in the configuration file or command-line arguments is read from environment variables prefixed with `RUSTYGPT_`.
   - Environment variables are case-insensitive.

5. **Static Defaults**:
   - If a value is not provided in the configuration file, command-line arguments, or environment variables, the backend uses static defaults.

### Example Behavior

- If `--config config.yaml` is provided and the file contains:

  ```yaml
  server_port: 9090
  ```

  The `server_port` will be set to `9090`.

- If the environment variable `RUSTYGPT_SERVER_PORT=8081` is set, it will only be used if the configuration file does not provide a value for `server_port`.

- If neither the file nor the environment variable provides a value, the static default (`8080`) is used.

### Error Handling

- If both `config.yaml` and `config.json` are found in the working directory and no `--config` argument is provided, the backend returns an error: `Ambiguous configuration. Config parameter required.`
- If a configuration value is invalid (e.g., `server_port` is not a number or out of range), an error is displayed, and the backend exits with a non-zero code.

## Dynamic Reloading

The backend supports dynamic reloading of configuration. This feature is implemented in the `config.rs` module and can be triggered via API or signals (to be implemented in future updates).
