# quartofmt: A Formatter for Quarto, Pandoc, and Markdown

[![Build and Test](https://github.com/jolars/quartofmt/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/jolars/quartofmt/actions/workflows/build-and-test.yml)

A CLI formatter for Quarto (`.qmd`), Pandoc, and Markdown files.

## Work in Progress

This project is in **very** early development. Expect bugs, missing features, and breaking changes.

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
quartofmt document.qmd | cat
```

## Configuration

quartofmt looks for a configuration in:

1. `.quartofmt.toml` or `quartofmt.toml` in current directory or parent directories
2. `~/.config/quartofmt/config.toml`

### Example config

```toml
line_width = 80
```

## Motivation

I wanted a formatter that understands Quarto and Pandoc syntax. I have tried
to use Prettier as well as mdformat, but both fail to handle some of
the particular syntax used in Quarto documents, such as fenced divs and
some of the table syntax.

## Design Goals

- Support Quarto, Pandoc, and Markdown syntax
- Be fast
- Be configurable, but have sane defaults (that most people can
  agree on)
- Format math

Notably, I don't expect to support formatting the code blocks or yaml
frontmatter. The primary reason for this is that it is now possible
to do this already by language injection through tree sitter, for instance,
which means that a good formatter should already be able to handle this.
