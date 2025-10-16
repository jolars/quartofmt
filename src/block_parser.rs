use crate::syntax::{SyntaxKind, SyntaxNode};
use rowan::GreenNodeBuilder;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

pub struct BlockParser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> BlockParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
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

    pub fn try_parse_fenced_code_block(&mut self) -> bool {
        log::debug!("Trying to parse fenced code block at position {}", self.pos);

        if self.pos >= self.lines.len() {
            return false;
        }

        let line = self.lines[self.pos];
        let trimmed = strip_leading_spaces(line);

        // Check if this is a fenced code block opening
        let (fence_char, fence_count) = if let Some(count) = get_fence_count(trimmed, '`') {
            ('`', count)
        } else if let Some(count) = get_fence_count(trimmed, '~') {
            ('~', count)
        } else {
            return false;
        };

        // Must have at least 3 fence characters
        if fence_count < 3 {
            return false;
        }

        // blank_before_header: require blank line before, unless at BOF
        if !self.has_blank_line_before() {
            return false;
        }

        // Extract info string (language, attributes, etc.)
        let info_string = trimmed[fence_count..].trim();

        // Start code block
        self.builder.start_node(SyntaxKind::CodeBlock.into());

        // Opening fence
        self.builder.start_node(SyntaxKind::CodeFenceOpen.into());
        self.builder
            .token(SyntaxKind::CodeFenceMarker.into(), &trimmed[..fence_count]);
        if !info_string.is_empty() {
            self.builder.token(SyntaxKind::CodeInfo.into(), info_string);
        }
        self.builder.finish_node(); // CodeFenceOpen

        self.pos += 1;

        // Collect content lines until we find a closing fence
        let mut content_lines = Vec::new();
        let mut found_closing = false;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed_line = strip_leading_spaces(line);

            // Check if this is a valid closing fence
            if let Some(closing_count) = get_fence_count(trimmed_line, fence_char)
                && closing_count >= fence_count
            {
                // Make sure the rest of the line is empty (or just whitespace)
                let after_fence = trimmed_line[closing_count..].trim();
                if after_fence.is_empty() {
                    found_closing = true;
                    break;
                }
            }

            content_lines.push(line);
            self.pos += 1;
        }

        // Add content
        if !content_lines.is_empty() {
            self.builder.start_node(SyntaxKind::CodeContent.into());
            for (i, content_line) in content_lines.iter().enumerate() {
                if i > 0 {
                    self.builder.token(SyntaxKind::NEWLINE.into(), "\n");
                }
                self.builder.token(SyntaxKind::TEXT.into(), content_line);
            }
            self.builder.finish_node(); // CodeContent
        }

        // Closing fence (if found)
        if found_closing {
            let closing_line = self.lines[self.pos];
            let closing_trimmed = strip_leading_spaces(closing_line);
            let closing_count = get_fence_count(closing_trimmed, fence_char).unwrap();

            self.builder.start_node(SyntaxKind::CodeFenceClose.into());
            self.builder.token(
                SyntaxKind::CodeFenceMarker.into(),
                &closing_trimmed[..closing_count],
            );
            self.builder.finish_node(); // CodeFenceClose

            self.pos += 1;
        }

        self.builder.finish_node(); // CodeBlock

        log::debug!("Parsed fenced code block, found_closing: {}", found_closing);
        true
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

        let flat_tree = SyntaxNode::new_root(self.builder.finish());

        // Second pass: resolve container blocks
        resolve_containers(flat_tree)
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

            if self.try_parse_fenced_code_block() {
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

fn resolve_containers(root: SyntaxNode) -> SyntaxNode {
    use rowan::GreenNodeBuilder;

    let mut builder = GreenNodeBuilder::new();

    // Copy the root node type
    builder.start_node(root.kind().into());

    // Process the document children
    if let Some(doc) = root.children().find(|n| n.kind() == SyntaxKind::DOCUMENT) {
        builder.start_node(SyntaxKind::DOCUMENT.into());
        resolve_container_children(&mut builder, &doc.children().collect::<Vec<_>>());
        builder.finish_node();
    }

    builder.finish_node();
    SyntaxNode::new_root(builder.finish())
}

fn resolve_container_children(builder: &mut GreenNodeBuilder<'static>, children: &[SyntaxNode]) {
    let mut i = 0;

    while i < children.len() {
        if let Some(blockquote_end) = try_identify_blockquote(children, i) {
            // Found blockquote pattern from i..blockquote_end
            build_blockquote_node(builder, &children[i..blockquote_end]);
            i = blockquote_end;
        } else {
            // Regular node, copy as-is
            copy_node_recursively(builder, &children[i]);
            i += 1;
        }
    }
}

fn try_identify_blockquote(children: &[SyntaxNode], start: usize) -> Option<usize> {
    if start >= children.len() {
        return None;
    }

    // Check if this paragraph looks like a blockquote (starts with >)
    let first_node = &children[start];
    if first_node.kind() != SyntaxKind::PARAGRAPH {
        return None;
    }

    let text = first_node.text().to_string();
    let first_line = text.lines().next().unwrap_or("");

    // Check if line has valid blockquote indentation (max 3 spaces before >)
    if !is_valid_blockquote_line(first_line) {
        return None;
    }

    // Find consecutive blockquote paragraphs and blank lines
    let mut end = start + 1;
    while end < children.len() {
        let node = &children[end];
        match node.kind() {
            SyntaxKind::PARAGRAPH => {
                let text = node.text().to_string();
                let first_line = text.lines().next().unwrap_or("");
                if is_valid_blockquote_line(first_line) {
                    end += 1;
                } else {
                    break;
                }
            }
            SyntaxKind::BlankLine => {
                // Blank lines can be part of blockquotes
                end += 1;
            }
            _ => break,
        }
    }

    Some(end)
}

fn is_valid_blockquote_line(line: &str) -> bool {
    // Check for up to 3 spaces, then >, following Pandoc spec
    if line.starts_with('>') {
        return true;
    }
    if line.starts_with(' ') && line.len() > 1 && line[1..].starts_with('>') {
        return true;
    }
    if line.starts_with("  ") && line.len() > 2 && line[2..].starts_with('>') {
        return true;
    }
    if line.starts_with("   ") && line.len() > 3 && line[3..].starts_with('>') {
        return true;
    }
    // 4 or more spaces before > is not a valid blockquote
    false
}

fn build_blockquote_node(builder: &mut GreenNodeBuilder<'static>, nodes: &[SyntaxNode]) {
    builder.start_node(SyntaxKind::BlockQuote.into());

    // Extract content from blockquote markers and recursively parse
    let mut content_lines = Vec::new();

    for node in nodes {
        match node.kind() {
            SyntaxKind::PARAGRAPH => {
                let text = node.text().to_string();
                for line in text.lines() {
                    let trimmed = line.trim_start();
                    if let Some(stripped) = trimmed.strip_prefix('>') {
                        // Remove '>' and optional space
                        let content = if stripped.starts_with(' ') {
                            &stripped[1..]
                        } else {
                            stripped
                        };
                        content_lines.push(content.to_string());
                    }
                }
            }
            SyntaxKind::BlankLine => {
                content_lines.push(String::new());
            }
            _ => {}
        }
    }

    if !content_lines.is_empty() {
        let content = content_lines.join("\n");
        if !content.trim().is_empty() {
            // Create a sub-parser for the blockquote content - this enables recursion!
            let sub_parser = BlockParser::new(&content);
            let sub_tree = sub_parser.parse();

            // Copy the sub-tree's document children into our blockquote
            if let Some(doc) = sub_tree
                .children()
                .find(|n| n.kind() == SyntaxKind::DOCUMENT)
            {
                for child in doc.children() {
                    copy_node_recursively(builder, &child);
                }
            }
        }
    }

    builder.finish_node();
}

fn copy_node_recursively(builder: &mut GreenNodeBuilder<'static>, node: &SyntaxNode) {
    builder.start_node(node.kind().into());

    for child in node.children_with_tokens() {
        match child {
            rowan::NodeOrToken::Node(n) => copy_node_recursively(builder, &n),
            rowan::NodeOrToken::Token(t) => {
                builder.token(t.kind().into(), t.text());
            }
        }
    }

    builder.finish_node();
}

fn strip_leading_spaces(line: &str) -> &str {
    line.strip_prefix("   ")
        .or_else(|| line.strip_prefix("  "))
        .or_else(|| line.strip_prefix(" "))
        .unwrap_or(line)
}

fn get_fence_count(line: &str, fence_char: char) -> Option<usize> {
    if !line.starts_with(fence_char) {
        return None;
    }

    let count = line.chars().take_while(|&c| c == fence_char).count();
    Some(count)
}

#[cfg(test)]
mod tests {
    mod blanklines;
    mod blockquotes;
    mod code_blocks;
    mod headings;
    mod helpers;
}
