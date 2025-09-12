use crate::config::Config;
use crate::syntax::{SyntaxKind, SyntaxNode};

pub struct Formatter {
    output: String,
    config: Config,
}

impl Formatter {
    pub fn new(config: Config) -> Self {
        Self {
            output: String::with_capacity(8192),
            config,
        }
    }

    fn wrap_text(&self, text: &str, width: usize) -> String {
        if text.trim().is_empty() {
            return String::new();
        }

        let options = textwrap::Options::new(width)
            .break_words(false)
            .word_separator(textwrap::WordSeparator::AsciiSpace)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);

        let words: Vec<&str> = text.split_whitespace().collect();
        let normalized = words.join(" ");

        textwrap::fill(&normalized, options)
    }

    pub fn format(mut self, node: &SyntaxNode) -> String {
        self.format_node(node, 0);
        self.output
    }

    fn format_node(&mut self, node: &SyntaxNode, indent: usize) {
        let line_width = self.config.line_width.unwrap();

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
                for child in node.children() {
                    match child.kind() {
                        SyntaxKind::PARAGRAPH => {
                            let text = child.text().to_string().trim().to_string();
                            let wrapped = textwrap::fill(&text, line_width.saturating_sub(2));
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
                let wrapped = self.wrap_text(&text, line_width);

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

            SyntaxKind::SimpleTable => {
                // Preserve table as-is, including line breaks and spacing
                self.output.push_str(&node.text().to_string());
                // for el in node.children_with_tokens() {
                //     match el {
                //         rowan::NodeOrToken::Token(t) => self.output.push_str(t.text()),
                //         rowan::NodeOrToken::Node(n) => self.output.push_str(&n.text().to_string()),
                //     }
                // }
            }

            SyntaxKind::InlineMath => {
                for child in node.children() {
                    self.output.push_str(&child.text().to_string());
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
                            line_width.saturating_sub(marker.len() + total_indent);
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
                        SyntaxKind::Label => {
                            label = Some(child.text().to_string().trim().to_string());
                        }
                        _ => {}
                    }
                }
                // Opening fence
                self.output.push_str("$$\n");
                // Math content
                if let Some(content) = math_content {
                    let math_indent = self.config.math_indent.unwrap();
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
