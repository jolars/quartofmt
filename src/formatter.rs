use crate::syntax::{SyntaxKind, SyntaxNode};

pub struct Formatter {
    output: String,
    line_width: usize,
}

impl Formatter {
    pub fn new(line_width: usize) -> Self {
        Self {
            output: String::new(),
            line_width,
        }
    }

    fn wrap_text(&self, text: &str, width: usize) -> String {
        if text.trim().is_empty() {
            return String::new();
        }

        // Normalize whitespace: split into words and rejoin with single spaces
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

        // Create custom line breaking options that prevent breaking at ](
        let options = textwrap::Options::new(width)
            .break_words(false)
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);

        // Replace ]( with a non-breaking sequence temporarily
        let protected = normalized.replace("](", "RIGHTBRACKET_LEFTPAREN");
        let wrapped = textwrap::fill(&protected, options);

        // Restore the original syntax
        wrapped.replace("RIGHTBRACKET_LEFTPAREN", "](")
    }

    pub fn format(mut self, node: &SyntaxNode) -> String {
        self.format_node(node, 0);
        self.output
    }

    fn format_node(&mut self, node: &SyntaxNode, indent: usize) {
        match node.kind() {
            SyntaxKind::ROOT | SyntaxKind::DOCUMENT => {
                for el in node.children_with_tokens() {
                    match el {
                        rowan::NodeOrToken::Node(n) => self.format_node(&n, indent),
                        rowan::NodeOrToken::Token(t) => match t.kind() {
                            SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {}
                            SyntaxKind::ImageLinkStart | SyntaxKind::LinkStart => {
                                self.output.push_str(t.text());
                            }
                            _ => self.output.push_str(t.text()),
                        },
                    }
                }
            }

            SyntaxKind::BlockQuote => {
                // Format children (paragraphs, blank lines) with > prefix
                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::PARAGRAPH => {
                            let text = child.text().to_string().trim().to_string();
                            let wrapped = textwrap::fill(&text, self.line_width.saturating_sub(2));
                            for line in wrapped.lines() {
                                self.output.push_str("> ");
                                self.output.push_str(line);
                                self.output.push('\n');
                            }
                        }
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
                let text = node.text().to_string();
                let wrapped = self.wrap_text(&text, self.line_width);

                if !wrapped.is_empty() {
                    self.output.push_str(&wrapped);
                }

                // Always end paragraphs with a newline for proper separation
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
            }

            SyntaxKind::List => {
                for child in node.children() {
                    self.format_node(&child, indent);
                }
            }

            SyntaxKind::ListItem => {
                let node_text = node.text().to_string();
                let local_indent = node_text.chars().take_while(|c| c.is_whitespace()).count();
                let total_indent = indent + local_indent;
                let trimmed = node_text.trim_start();
                if let Some(marker_end) = trimmed.find(' ') {
                    let marker = &trimmed[..marker_end + 1];
                    let content = trimmed[marker_end + 1..].trim();
                    if !content.is_empty() {
                        let available_width =
                            self.line_width.saturating_sub(marker.len() + total_indent);
                        let wrapped = self.wrap_text(content, available_width);

                        for (i, line) in wrapped.lines().enumerate() {
                            if i == 0 {
                                self.output.push_str(&" ".repeat(total_indent));
                                self.output.push_str(marker);
                            } else {
                                self.output
                                    .push_str(&" ".repeat(total_indent + marker.len()));
                            }
                            self.output.push_str(line);
                            self.output.push('\n');
                        }
                    }
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

            SyntaxKind::CodeBlock | SyntaxKind::MathBlock | SyntaxKind::FRONTMATTER => {
                // Preserve these blocks as-is
                let text = node.text().to_string();
                self.output.push_str(&text);
                // Ensure code blocks end with a newline for proper separation
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

pub fn format_tree(tree: &SyntaxNode, line_width: usize) -> String {
    Formatter::new(line_width).format(tree)
}
