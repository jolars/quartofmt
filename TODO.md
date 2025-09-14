Immediate correctness fixes

- Panic guards: advance_while() and tokenize() iteration caps will panic on
  long inputs. Remove in release builds or convert to debug_assert!/logging.
- Fenced div parsing: parse_fenced_div() calls parse_document(), which nests
  DOCUMENT nodes inside div content; formatter compensates but it’s awkward. Add
  a parse_blocks() that parses a sequence of blocks without wrapping in DOCUMENT.

Formatter improvements

- Wrap mode config is unused. Implement:
  - Off: preserve paragraph whitespace/newlines.
  - Soft: current wrapping behavior.
  - Hard: also break overlong tokens.
- Paragraph formatting currently uses node.text() and normalizes whitespace
  across everything, which can mangle inline constructs (links, inline code,
  inline math, LaTeX commands). Prefer walking children tokens:
  - Keep inline spans (InlineMath, LatexCommand, links, images, inline code) as
    atomic units when reflowing.
  - Only collapse/rewrap TEXT and WHITESPACE between inline spans.
- Lists: Current ListItem formatting derives marker/indent from raw text;
  fragile for numbered lists and nested mixes. Use the parsed structure (e.g.,
  capture marker and following space as explicit tokens/nodes) so you can compute
  hanging indents precisely and keep wrapped lines aligned.
- Avoid emitting extra trailing newlines (audit nodes that push a newline
  unconditionally).

Parser/lexer coverage to add

- Links/images fully (closing ’]’, ’(…)’), autolinks, and reference-style links. Or at least treat [..](..) as atomic for wrapping.
- HTML blocks and inline HTML beyond comments.
- Headings, ATX/Setext.
- Thematic breaks (---, \*\*\*, \_\_\_) vs table underlines.
- Nested lists with mixed bullets and numbers; list continuation lines.
- Block quotes with nested lists/code blocks.
- Escapes and entities.

Testing and quality

- idempotency tests (format twice == once).
- Fuzzing (cargo-fuzz) and corpus from Quarto docs.
- Property tests for tokenization invariants (concatenated token text == input).
- add a few big documents for performance smoke tests.

Config/CLI/editor integration

- Provide CLI: quartofmt [--check] [--write] [--config PATH] [--stdin|PATHS].
- Neovim: expose a robust CLI with --stdin --stdout for formatprg or provide an LSP/formatter endpoint.

Architecture polish

- Split parse_document() into parse_blocks() + parse_document() (outer wrapper).
- Keep DivInfo node (you defined it) and populate it; same for CodeInfo vs including newline in fence nodes.
- Expose resolved config getters to avoid unwraps.

Performance

- Benchmark wrapping and parsing on large files (cargo bench); preallocate buffers based on input size.

Small targeted fixes to prioritize

- Restrict list marker lexing to BOL.
