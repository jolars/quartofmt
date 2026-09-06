# panache-parser

Lossless CST parser and typed syntax wrappers for Pandoc Markdown, Quarto, and R
Markdown.

## Status

This crate is extracted from the Panache project and is evolving alongside it.
The API is still early and may change between releases.

## Usage

```rust
use panache_parser::syntax::BlockNode;
use panache_parser::{Flavor, ParserOptions, parse_document};

let parsed = parse_document(
    "# Heading\n\nParagraph text.",
    Some(ParserOptions::for_flavor(Flavor::Gfm)),
);

for block in parsed.document().block_nodes() {
    match block {
        BlockNode::Heading(heading) => println!("level {}", heading.level()),
        BlockNode::Unknown(node) => println!("unsupported: {}", node.syntax_name()),
        _ => {}
    }
}
```

`parse_document` is the consumer-oriented entry point. It returns a typed
document root together with embedded-language errors. `BlockNode` and
`InlineNode` classify semantic children, retain trivia explicitly, and expose
unknown syntax with its original kind, source, and byte range. Typed wrappers
provide attributes, link destinations, table alignment, GFM alerts, Quarto
callouts, embedded YAML, and executable-cell source and option provenance.

The CST remains lossless, and all ranges use zero-based, half-open byte offsets
into the original source. Consumers should translate these views into their own
document model. The `to_pandoc_ast` and `to_pandoc_json` functions are
Pandoc-conformance projectors, not a stable application IR or rendering
contract.

## Documentation

- API docs: <https://docs.rs/panache-parser>
- Project: <https://github.com/jolars/panache>
