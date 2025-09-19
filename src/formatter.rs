use crate::config::{Config, WrapMode};
use crate::syntax::{SyntaxKind, SyntaxNode};

use rowan::NodeOrToken;
use textwrap::wrap_algorithms::WrapAlgorithm;

pub struct Formatter {
    output: String,
    config: Config,
}

fn is_block_element(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::PARAGRAPH
            | SyntaxKind::List
            | SyntaxKind::BlockQuote
            | SyntaxKind::MathBlock
            | SyntaxKind::CodeBlock
            | SyntaxKind::SimpleTable
    )
}

impl Formatter {
    pub fn new(config: Config) -> Self {
        Self {
            output: String::with_capacity(8192),
            config,
        }
    }

    fn build_words<'a>(
        &self,
        node: &SyntaxNode,
        arena: &'a mut Vec<Box<str>>,
    ) -> Vec<textwrap::core::Word<'a>> {
        let mut piece_idx: Vec<usize> = Vec::new();
        let mut whitespace_after: Vec<bool> = Vec::new();
        let mut last_piece_pos: Option<usize> = None;
        let mut pending_space = false;

        let mut it = node.children_with_tokens();
        while let Some(el) = it.next() {
            match el {
                rowan::NodeOrToken::Token(t) => match t.kind() {
                    SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                        pending_space = true;
                    }
                    SyntaxKind::Link | SyntaxKind::ImageLink => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        arena.push(Box::<str>::from(t.text()));
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                    SyntaxKind::TEXT | SyntaxKind::LatexCommand | SyntaxKind::CodeSpan => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        arena.push(Box::<str>::from(t.text()));
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                    SyntaxKind::InlineMathMarker => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        let mut acc = String::new();
                        acc.push_str(t.text());
                        for next in it.by_ref() {
                            match next {
                                rowan::NodeOrToken::Token(nt) => {
                                    acc.push_str(nt.text());
                                    if nt.kind() == SyntaxKind::InlineMathMarker {
                                        break;
                                    }
                                }
                                rowan::NodeOrToken::Node(n) => {
                                    acc.push_str(&n.text().to_string());
                                }
                            }
                        }
                        arena.push(acc.into_boxed_str());
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                    _ => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        arena.push(Box::<str>::from(t.text()));
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                },
                rowan::NodeOrToken::Node(n) => match n.kind() {
                    SyntaxKind::InlineMath => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        arena.push(n.text().to_string().into_boxed_str());
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                    _ => {
                        if pending_space {
                            if let Some(prev) = last_piece_pos {
                                whitespace_after[prev] = true;
                            }
                            pending_space = false;
                        }
                        arena.push(n.text().to_string().into_boxed_str());
                        let idx = arena.len() - 1;
                        piece_idx.push(idx);
                        whitespace_after.push(false);
                        last_piece_pos = Some(piece_idx.len() - 1);
                    }
                },
            }
        }

        let mut words: Vec<textwrap::core::Word<'a>> = Vec::with_capacity(piece_idx.len());
        for (i, &idx) in piece_idx.iter().enumerate() {
            let s: &'a str = &arena[idx];
            println!("Word: {:?}", s);
            let mut w = textwrap::core::Word::from(s);

            if whitespace_after.get(i).copied().unwrap_or(false) {
                w.whitespace = " ";
            }
            words.push(w);
        }
        words
    }

    fn wrapped_lines_for_paragraph(&self, node: &SyntaxNode, width: usize) -> Vec<String> {
        let mut arena: Vec<Box<str>> = Vec::new();
        let words = self.build_words(node, &mut arena);

        let algo = WrapAlgorithm::new();
        let line_widths = [width];
        let lines = algo.wrap(&words, &line_widths);

        let mut out_lines = Vec::with_capacity(lines.len());

        for line in lines {
            let mut acc = String::new();
            for (i, w) in line.iter().enumerate() {
                acc.push_str(w.word);
                if i + 1 < line.len() {
                    acc.push_str(w.whitespace);
                } else {
                    acc.push_str(w.penalty);
                }
            }
            out_lines.push(acc);
        }
        out_lines
    }

    pub fn format(mut self, node: &SyntaxNode) -> String {
        self.format_node(node, 0);
        self.output
    }

    fn format_node(&mut self, node: &SyntaxNode, indent: usize) {
        let line_width = self.config.line_width;

        match node.kind() {
            SyntaxKind::ROOT | SyntaxKind::DOCUMENT => {
                for el in node.children_with_tokens() {
                    match el {
                        rowan::NodeOrToken::Node(n) => self.format_node(&n, indent),
                        rowan::NodeOrToken::Token(t) => match t.kind() {
                            SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {}
                            SyntaxKind::ImageLinkStart
                            | SyntaxKind::LinkStart
                            | SyntaxKind::LatexCommand => {
                                self.output.push_str(t.text());
                            }
                            _ => self.output.push_str(t.text()),
                        },
                    }
                }
            }

            SyntaxKind::Heading => {
                // Determine level
                let mut level = 1;
                let mut content = String::new();
                let mut saw_content = false;

                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::AtxHeadingMarker => {
                            let t = child.text().to_string();
                            level = t.chars().take_while(|&c| c == '#').count().clamp(1, 6);
                        }
                        SyntaxKind::SetextHeadingUnderline => {
                            let t = child.text().to_string();
                            if t.chars().all(|c| c == '=') {
                                level = 1;
                            } else {
                                level = 2;
                            }
                        }
                        SyntaxKind::HeadingContent => {
                            let mut t = child.text().to_string();
                            // Trim trailing spaces and closing hashes in ATX form
                            t = t.trim_end().to_string();
                            // Remove trailing " ###" if present
                            let trimmed_hash = t.trim_end_matches('#').to_string();
                            if trimmed_hash.len() != t.len() {
                                t = trimmed_hash.trim_end().to_string();
                            }
                            // Normalize internal newlines
                            content = t.trim().to_string();
                            saw_content = true;
                        }
                        _ => {}
                    }
                }
                if !saw_content {
                    content = node.text().to_string();
                }
                self.output.push_str(&"#".repeat(level));
                self.output.push(' ');
                self.output.push_str(&content);
                self.output.push('\n');

                if let Some(next) = node.next_sibling()
                    && is_block_element(next.kind())
                    && !self.output.ends_with("\n\n")
                {
                    self.output.push('\n');
                }
            }

            SyntaxKind::LatexEnvironment => {
                // // Output the environment exactly as written
                let text = node.text().to_string();
                self.output.push_str(&text);
                if !text.ends_with('\n') {
                    self.output.push('\n');
                }
            }

            SyntaxKind::Comment => {
                let text = node.text().to_string();
                self.output.push_str(&text);
                if !text.ends_with('\n') {
                    self.output.push('\n');
                }
            }

            SyntaxKind::LatexCommand => {
                // Standalone LaTeX commands - preserve exactly as written
                let text = node.text().to_string();
                self.output.push_str(&text);
                // Don't add extra newlines for standalone LaTeX commands
            }

            SyntaxKind::BlockQuote => {
                // Format children (paragraphs, blank lines) with > prefix
                let wrap_mode = self.config.wrap.clone().unwrap_or(WrapMode::Reflow);

                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::PARAGRAPH => match wrap_mode {
                            WrapMode::Preserve => {
                                let text = child.text().to_string();
                                for line in text.lines() {
                                    self.output.push_str("> ");
                                    self.output.push_str(line);
                                    self.output.push('\n');
                                }
                            }
                            WrapMode::Reflow => {
                                let width = self.config.line_width.saturating_sub(2);
                                let lines = self.wrapped_lines_for_paragraph(&child, width);
                                for line in lines {
                                    self.output.push_str("> ");
                                    self.output.push_str(&line);
                                    self.output.push('\n');
                                }
                            }
                        },
                        SyntaxKind::BlankLine => {
                            self.output.push_str(">\n");
                        }
                        _ => {
                            // Handle other content within block quotes
                            self.format_node(&child, indent);
                        }
                    }
                }
            }

            SyntaxKind::PARAGRAPH => {
                let wrap_mode = self.config.wrap.clone().unwrap_or(WrapMode::Reflow);
                match wrap_mode {
                    WrapMode::Preserve => {
                        let text = node.text().to_string();
                        self.output.push_str(&text);
                        if !self.output.ends_with('\n') {
                            self.output.push('\n');
                        }
                    }
                    WrapMode::Reflow => {
                        let lines = self.wrapped_lines_for_paragraph(node, line_width);

                        for (i, line) in lines.iter().enumerate() {
                            if i > 0 {
                                self.output.push('\n');
                            }
                            self.output.push_str(line);
                        }

                        if !self.output.ends_with('\n') {
                            self.output.push('\n');
                        }
                    }
                }
            }

            SyntaxKind::List => {
                for child in node.children() {
                    self.format_node(&child, indent);
                }
            }

            SyntaxKind::SimpleTable => {
                // Preserve table as-is, including line breaks and spacing
                self.output.push_str(&node.text().to_string());
            }

            SyntaxKind::InlineMath => {
                for child in node.children() {
                    self.output.push_str(&child.text().to_string());
                }
            }

            SyntaxKind::ListItem => {
                // Compute indent and marker from leading tokens
                let mut marker = String::new();
                let mut local_indent = 0;
                let mut content_start = false;

                for el in node.children_with_tokens() {
                    match el {
                        NodeOrToken::Token(t) => match t.kind() {
                            SyntaxKind::WHITESPACE => {
                                if !content_start {
                                    local_indent += t.text().len();
                                }
                            }
                            SyntaxKind::ListMarker => {
                                marker = t.text().to_string();
                                content_start = true;
                            }
                            _ => {
                                content_start = true;
                            }
                        },
                        _ => {
                            content_start = true;
                        }
                    }
                }

                let total_indent = indent + local_indent;
                let hanging = marker.len() + 1 + total_indent; // +1 for the space after marker
                let available_width = self.config.line_width.saturating_sub(hanging);

                // Build words from the whole node, then drop the leading marker word
                let mut arena: Vec<Box<str>> = Vec::new();
                let mut words = self.build_words(node, &mut arena);
                if let Some(first) = words.first()
                    && first.word == marker
                {
                    // Remove the marker; we will print it ourselves with a following space
                    words.remove(0);
                }

                let algo = WrapAlgorithm::new();
                let line_widths = [available_width];
                let lines = algo.wrap(&words, &line_widths);

                for (i, line) in lines.iter().enumerate() {
                    if i == 0 {
                        self.output.push_str(&" ".repeat(total_indent));
                        self.output.push_str(&marker);
                        self.output.push(' ');
                    } else {
                        // Hanging indent includes marker + one space
                        self.output
                            .push_str(&" ".repeat(total_indent + marker.len() + 1));
                    }
                    for (j, w) in line.iter().enumerate() {
                        self.output.push_str(w.word);
                        if j + 1 < line.len() {
                            self.output.push_str(w.whitespace);
                        } else {
                            self.output.push_str(w.penalty);
                        }
                    }
                    self.output.push('\n');
                }

                // Format nested lists inside this list item with increased indent
                for child in node.children() {
                    if child.kind() == SyntaxKind::List {
                        self.format_node(&child, total_indent + 2);
                    }
                }
            }

            SyntaxKind::FencedDiv => {
                let mut fence_open = None;
                let mut fence_close = None;
                let mut div_content = None;

                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::DivFenceOpen => {
                            fence_open = Some(child.text().to_string());
                        }
                        SyntaxKind::DivContent => {
                            div_content = Some(child);
                        }
                        SyntaxKind::DivFenceClose => {
                            fence_close = Some(child.text().to_string());
                        }
                        _ => {}
                    }
                }

                if let Some(open) = fence_open {
                    self.output.push_str(&open);
                }
                if let Some(content_node) = div_content {
                    for grandchild in content_node.children() {
                        if grandchild.kind() == SyntaxKind::DOCUMENT {
                            for doc_child in grandchild.children() {
                                self.format_node(&doc_child, indent);
                            }
                        } else {
                            self.format_node(&grandchild, indent);
                        }
                    }
                }
                if let Some(close) = fence_close {
                    self.output.push_str(&close);
                    if !close.ends_with('\n') {
                        self.output.push('\n');
                    }
                }
            }

            SyntaxKind::InlineMathMarker => {
                // Output inline math as $...$ or $$...$$ (on the same line)
                self.output.push_str(node.text().to_string().trim());
            }

            SyntaxKind::MathBlock => {
                let mut label = None;
                let mut math_content = None;
                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::MathContent => {
                            math_content = Some(child.text().to_string());
                        }
                        SyntaxKind::Attribute => {
                            label = Some(child.text().to_string().trim().to_string());
                        }
                        _ => {}
                    }
                }
                // Opening fence
                self.output.push_str("$$\n");
                // Math content
                if let Some(content) = math_content {
                    let math_indent = self.config.math_indent;
                    for line in content.trim().lines() {
                        self.output.push_str(&" ".repeat(math_indent));
                        self.output.push_str(line.trim_end());
                        self.output.push('\n');
                    }
                }
                // Closing fence (with label if present)
                self.output.push_str("$$");
                if let Some(lbl) = label {
                    self.output.push(' ');
                    self.output.push_str(&lbl);
                }
                self.output.push('\n');
            }

            SyntaxKind::CodeBlock | SyntaxKind::FRONTMATTER => {
                // Preserve these blocks as-is
                let text = node.text().to_string();
                self.output.push_str(&text);
                // Ensure these blocks end with appropriate spacing
                if !text.ends_with('\n') {
                    self.output.push('\n');
                }
            }

            SyntaxKind::BlankLine => {
                // Preserve the actual blank line content (multiple newlines/whitespace)
                self.output.push_str(&node.text().to_string());
            }

            _ => {
                // Fallback: append node text (should be rare with children_with_tokens above)
                self.output.push_str(&node.text().to_string());
            }
        }
    }
}

pub fn format_tree(tree: &SyntaxNode, config: &Config) -> String {
    Formatter::new(config.clone()).format(tree)
}
