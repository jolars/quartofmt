use crate::syntax::{SyntaxKind, SyntaxNode};
use rowan::GreenNodeBuilder;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub struct BlockParser<'a> {
    input: &'a str,
    lines: Vec<&'a str>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> BlockParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            input,
            lines,
            pos: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    fn has_blank_line_before(&self) -> bool {
        if self.pos == 0 {
            true
        } else {
            self.lines[self.pos - 1].trim().is_empty()
        }
    }

    fn try_parse_atx_heading(&mut self) -> bool {
        log::debug!("Trying to parse ATX heading at position {}", self.pos);

        if self.pos >= self.lines.len() {
            return false;
        }
        let line = self.lines[self.pos];

        // Allow up to 3 leading spaces
        let trimmed = strip_leading_spaces(line);

        // Must start with 1-6 '#'s
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if hashes == 0 || hashes > 6 {
            return false;
        }

        // Must be followed by a space (Pandoc: space_in_atx_header)
        let after_hashes = &trimmed[hashes..];
        if !after_hashes.starts_with(' ') {
            return false;
        }

        // blank_before_header: require blank line before, unless at BOF
        if !self.has_blank_line_before() {
            return false;
        }

        // The rest after hashes is the content (may have trailing hashes)
        let mut content = after_hashes.trim_start();
        // Remove optional trailing hashes and spaces
        if let Some(idx) = content.rfind(|c| c != '#' && c != ' ') {
            content = content[..=idx].trim_end();
        } else {
            content = "";
        }

        // Emit nodes
        self.builder.start_node(SyntaxKind::Heading.into());

        // Marker node for the hashes
        self.builder.start_node(SyntaxKind::AtxHeadingMarker.into());
        self.builder
            .token(SyntaxKind::AtxHeadingMarker.into(), &trimmed[..hashes]);
        self.builder.finish_node();

        // Heading content node
        self.builder.start_node(SyntaxKind::HeadingContent.into());
        self.builder.token(SyntaxKind::TEXT.into(), content);
        self.builder.finish_node();

        self.builder.finish_node(); // Heading

        self.pos += 1;
        true
    }

    pub fn try_parse_blank_line(&mut self) -> bool {
        log::debug!("Trying to parse blank line at position {}", self.pos);

        if self.pos >= self.lines.len() {
            return false;
        }

        let line = self.lines[self.pos];

        if line.trim().is_empty() {
            self.builder.start_node(SyntaxKind::BlankLine.into());
            self.builder.token(SyntaxKind::BlankLine.into(), line);
            self.builder.finish_node();
            self.pos += 1;

            log::debug!("Parsed blank line at position {}", self.pos);

            return true;
        }

        false
    }

    pub fn try_parse_paragraph(&mut self) -> bool {
        log::debug!("Trying to parse paragraph at position {}", self.pos);

        if self.pos >= self.lines.len() {
            return false;
        }
        let line = self.lines[self.pos];

        if line.trim().is_empty() {
            return false;
        }

        // Start paragraph node
        self.builder.start_node(SyntaxKind::PARAGRAPH.into());

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim().is_empty() {
                break;
            }

            // Add line as TEXT token (could be improved to handle inline elements)
            self.builder.token(SyntaxKind::TEXT.into(), line);
            self.builder.token(SyntaxKind::NEWLINE.into(), "\n");

            self.pos += 1;

            log::debug!("Added line to paragraph: {}", line);
        }

        self.builder.finish_node(); // PARAGRAPH

        true
    }

    pub fn parse(mut self) -> SyntaxNode {
        #[cfg(debug_assertions)]
        {
            init_logger();
        }

        self.builder.start_node(SyntaxKind::ROOT.into());
        self.parse_document(); // <-- Add this line!
        self.builder.finish_node();

        SyntaxNode::new_root(self.builder.finish())
    }

    fn parse_document(&mut self) {
        self.builder.start_node(SyntaxKind::DOCUMENT.into());

        log::debug!("Starting document parse");

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            log::debug!("Parsing line {}: {}", self.pos + 1, line);

            if self.try_parse_blank_line() {
                continue;
            }

            if self.try_parse_atx_heading() {
                continue;
            }

            if self.try_parse_paragraph() {
                continue;
            }

            // If no other block matched, just skip the line (could be improved)
            self.pos += 1;
        }

        self.builder.finish_node();
    }
}

fn strip_leading_spaces(line: &str) -> &str {
    line.strip_prefix("   ")
        .or_else(|| line.strip_prefix("  "))
        .or_else(|| line.strip_prefix(" "))
        .unwrap_or(line)
}

#[cfg(test)]
mod tests {
    mod blanklines;
    mod headings;
    mod helpers;
}
