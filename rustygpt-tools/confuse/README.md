# ConFuse

ConFuse is a command-line tool for running multiple tasks concurrently with log prefixing (in a style similar to concurrently). It allows you to run multiple commands simultaneously, with each command's output prefixed by its unique colored identifier.

## Features

- Run multiple commands concurrently.
- Color-coded log prefixes for easy differentiation.
- Inline naming and working directory specification using the "@" delimiter.
- Graceful shutdown on SIGTERM/SIGINT (Unix systems).

## Installation

Make sure you have Rust installed. Then, clone the repository and build the project:

```bash
git clone <repository-url>
cd /Users/vanna/Source/rusty_gpt
cargo build --release --package confuse
```

You can then add the binary to your PATH or run it directly from the target directory:

```bash
./target/release/confuse [OPTIONS] <commands>...
```

## Usage

ConFuse accepts multiple commands as positional arguments. Each command may include an inline prefix to specify a custom task name and working directory.

### Command format

- Basic command (no inline naming):

  ```
  confuse "cargo watch -x run" "trunk watch"
  ```

- With inline naming and working directory:
  ```
  confuse "api@./backend:cargo run" "web@./frontend:trunk serve"
  ```

(Note: To set a working directory inline, use the "@" delimiter between the name and directory. A prefix without "@" will be used solely as the task name.)

### CLI Options

- `-n, --names <NAMES>` A comma-separated list of names to override inline command names.

- `-c, --prefix-colors <PREFIX_COLORS>` A comma-separated list of colors for log prefixes. Valid colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`.

- `-c, --cwd <CWD>` An optional working directory to apply to all commands.

## Examples

Run two commands concurrently:

```bash
confuse "cargo watch -x run" "trunk watch"
```

Run commands with custom task names:

```bash
confuse --names "Backend,Frontend" "cargo watch -x run" "trunk watch"
```

Run commands with specific prefix colors:

```bash
confuse --prefix-colors "blue,red" "cargo watch -x run" "trunk watch"
```
