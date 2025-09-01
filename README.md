# quartofmt: A Formatter for Quarto, Pandoc, and Markdown

A CLI formatter for Quarto (`.qmd`), Pandoc, and Markdown files.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Format a file and output to stdout
quartofmt document.qmd

# Format a file in place
quartofmt --write document.qmd

# Check if a file is formatted
quartofmt --check document.qmd

# Format from stdin
cat document.qmd | quartofmt
```

## Configuration

quartofmt looks for a configuration in:

1. `.quartofmt.toml` or `quartofmt.toml` in current directory or parent directories
2. `~/.config/quartofmt/config.toml`

### Example config

```toml
line_width = 80
```
