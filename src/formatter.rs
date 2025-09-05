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

    pub fn format(mut self, node: &SyntaxNode) -> String {
        self.format_node(node);
        self.output
    }

    fn format_node(&mut self, node: &SyntaxNode) {
        match node.kind() {
            SyntaxKind::ROOT | SyntaxKind::DOCUMENT => {
                for el in node.children_with_tokens() {
                    match el {
                        rowan::NodeOrToken::Node(n) => self.format_node(&n),
                        rowan::NodeOrToken::Token(t) => {
                            // Only preserve structural tokens, not whitespace
                            match t.kind() {
                                SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                                    // Skip these - they'll be handled by proper formatting
                                }
                                _ => self.output.push_str(t.text()),
                            }
                        }
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
                            self.format_node(&child);
                        }
                    }
                }
            }

            SyntaxKind::PARAGRAPH => {
                let text = node.text().to_string();

                // Normalize whitespace: split into words and rejoin with single spaces
                let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");

                if !normalized.is_empty() {
                    let wrapped = textwrap::fill(&normalized, self.line_width);
                    self.output.push_str(&wrapped);
                }

                // Always end paragraphs with a newline for proper separation
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
            }

            SyntaxKind::CodeBlock
            | SyntaxKind::FencedDiv
            | SyntaxKind::MathBlock
            | SyntaxKind::FRONTMATTER => {
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
