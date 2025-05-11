# i18n-agent

i18n-agent is a command-line tool for managing internationalization (i18n) translation files in your project. It scans your code for translation key usage, audits your translation files to detect missing or unused keys, cleans obsolete entries, and generates detailed reports in various formats. The tool also helps create templates for missing translations and merge external language files with the reference.

## Features

- **Scan** Recursively scan the codebase for translation key usage using patterns like `i18n.t("key")` and `i18n.translate("key")`.

- **Audit** Compare keys used in the codebase against the keys present in JSON translation files. Identify unused keys and missing translations relative to the reference language (default: `en`).

- **Clean** Remove unused keys from translation files (with backup support) to reduce clutter and keep translations up to date.

- **Report** Generate a detailed translation report in text, JSON, or HTML format. The report covers key usage statistics, unused keys and missing translations for each language.

- **Template** Create template files for missing translations based on the reference language. The generated templates include the reference value as a hint.

- **Merge** Merge a language’s translation file with the reference language. Translated strings override the reference values while missing keys are clearly flagged.

## Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   ```
2. Navigate to the `tools/i18n-agent` directory:
   ```bash
   cd rusty_gpt/tools/i18n-agent
   ```
3. Build the tool using Cargo:
   ```bash
   cargo build --release
   ```

## Usage

Run `i18n-agent` with one of the available commands:

### Scan

Scans your code for translation key usage.

```bash
cargo run -- scan --src path/to/your/source
```

### Audit

Audits your translation files and prints a summary report.

```bash
cargo run -- audit --src path/to/your/source --trans path/to/translations [--format text|json|html]
```

### Clean

Removes unused translation keys. Use `--backup` to create backups before cleaning.

```bash
cargo run -- clean --src path/to/your/source --trans path/to/translations --backup
```

### Report

Generates a detailed report in the specified format.

```bash
cargo run -- report --src path/to/your/source --trans path/to/translations [--output output_directory] --format html
```

### Template

Generates template files for missing translations. Templates are created relative to the translations directory (or in a custom output directory).

```bash
cargo run -- template --src path/to/your/source --trans path/to/translations [--output output_directory]
```

### Merge

Creates a merged translation file for a specific target language, using the reference language as a base. Missing keys are marked.

```bash
cargo run -- merge --lang es --src path/to/your/source --trans path/to/translations [--output output_directory]
```

## Configuration Options

- **--src** Path to the source directory to scan. The default is `frontend/src`.

- **--trans** Path to the translations directory. The default is `frontend/translations`.

- **--output** Output directory for reports and templates. If not specified, the current directory or translations directory is used.

- **--verbose** Enables detailed log output during scanning.

- **--backup** Creates backups of translation files before making any modifications.

- **--format** Selects the report format: `text` (default), `json`, or `html`.

## Code Structure

- **main.rs** Handles CLI parsing, path validations, and delegating commands to appropriate modules.

- **scanner.rs** Contains logic to scan code files for translation key usage using regular expressions.

- **analyzer.rs** Audits translation files against used keys and provides functions for extracting keys, calculating coverage, and identifying missing translations.

- **reporter.rs** Provides functions to generate and print audit reports in different formats.

- **generator.rs** Implements cleaning of translation files, creating backups, generating missing translation templates, and merging translations.

## Testing and Contribution

The tool includes comprehensive tests located alongside the source files. To run the tests:

```bash
just i18n-test
```

Contributions are welcome. Please open an issue or submit a pull request for any feature, bug fix, or improvement.

## Additional Information

For further details on how the tool works or to report issues, please see the project's repository or contact the maintainers.
