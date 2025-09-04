use crate::lexer::{Token, tokenize};
use crate::syntax::{SyntaxKind, SyntaxNode};
use rowan::GreenNodeBuilder;

pub struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let tokens = tokenize(input);
        Self {
            input,
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    pub fn parse(mut self) -> SyntaxNode {
        self.builder.start_node(SyntaxKind::ROOT.into());
        self.parse_document();
        self.builder.finish_node();

        SyntaxNode::new_root(self.builder.finish())
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        if let Some(token) = self.current_token() {
            let start = self.byte_offset();
            let end = start + token.len;
            let text = &self.input[start..end];

            log::trace!("Advancing: {:?} = {:?}", token.kind, text);
            self.builder.token(token.kind.into(), text);
            self.pos += 1;
        } else {
            log::trace!("Advance called but no current token");
        }
    }

    fn byte_offset(&self) -> usize {
        self.tokens[..self.pos].iter().map(|token| token.len).sum()
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current_token()
            .map(|token| token.kind == kind)
            .unwrap_or(false)
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn parse_document(&mut self) {
        self.builder.start_node(SyntaxKind::DOCUMENT.into());

        log::debug!("Starting document parse");

        // Uncomment for debugging:
        self.debug_tokens();

        // Check for frontmatter at the beginning
        if self.at(SyntaxKind::FrontmatterDelim) {
            self.parse_frontmatter();
        }

        let mut iterations = 0;
        while !self.at_eof() {
            iterations += 1;
            if iterations > 1000 {
                panic!(
                    "Too many iterations in parse_document! Current token: {:?} at pos {pos}",
                    self.current_token(),
                    pos = self.pos
                );
            }

            log::trace!(
                "Parse iteration {iterations}: pos={}, token={:?}",
                self.pos,
                self.current_token()
            );

            let old_pos = self.pos;
            match self.current_token().map(|t| t.kind) {
                Some(SyntaxKind::FenceMarker) => self.parse_code_block(),
                Some(SyntaxKind::DivMarker) => self.parse_fenced_div(),
                Some(SyntaxKind::MathMarker) => self.parse_math_block(),
                Some(SyntaxKind::NEWLINE) if self.is_blank_line() => self.parse_blank_line(),
                Some(SyntaxKind::WHITESPACE) => {
                    // Skip standalone whitespace
                    self.advance();
                }
                Some(SyntaxKind::NEWLINE) => {
                    // Skip standalone newlines that aren't blank lines
                    self.advance();
                }
                _ => self.parse_paragraph(),
            }

            // Safety check: ensure we always advance
            if self.pos == old_pos && !self.at_eof() {
                panic!(
                    "Parser stuck! Not advancing from pos {} with token {:?}",
                    self.pos,
                    self.current_token()
                );
            }
        }

        self.builder.finish_node();
    }

    fn parse_frontmatter(&mut self) {
        self.builder.start_node(SyntaxKind::FRONTMATTER.into());

        log::debug!("Starting frontmatter parse at pos {}", self.pos);

        // Opening delimiter
        if self.at(SyntaxKind::FrontmatterDelim) {
            log::debug!("Found opening delimiter");
            self.advance(); // ---/+++
        } else {
            panic!("Expected frontmatter delimiter at start of parse_frontmatter");
        }

        // Skip to end of line after opening delimiter
        while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }
        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }

        // Content until closing delimiter
        let mut content_iterations = 0;
        while !self.at_eof() && !self.at(SyntaxKind::FrontmatterDelim) {
            content_iterations += 1;
            if content_iterations > 100 {
                panic!(
                    "Too many iterations in frontmatter content! Current token: {:?}",
                    self.current_token()
                );
            }
            self.advance();
        }

        // Closing delimiter
        if self.at(SyntaxKind::FrontmatterDelim) {
            log::debug!("Found closing delimiter");
            self.advance();
            // Skip to end of line after closing delimiter
            while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
                self.advance();
            }
            if self.at(SyntaxKind::NEWLINE) {
                self.advance();
            }
        }

        log::debug!("Finished frontmatter parse at pos {}", self.pos);
        self.builder.finish_node();
    }

    fn parse_code_block(&mut self) {
        self.builder.start_node(SyntaxKind::CodeBlock.into());

        // Opening fence
        self.builder.start_node(SyntaxKind::CodeFenceOpen.into());
        self.advance(); // fence marker

        // Code info (language, options)
        if self.at(SyntaxKind::TEXT) || self.at(SyntaxKind::WHITESPACE) {
            self.builder.start_node(SyntaxKind::CodeInfo.into());
            while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
                self.advance();
            }
            self.builder.finish_node();
        }

        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }
        self.builder.finish_node();

        // Code content
        self.builder.start_node(SyntaxKind::CodeContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::FenceMarker) {
            self.advance();
        }
        self.builder.finish_node();

        // Closing fence
        if self.at(SyntaxKind::FenceMarker) {
            self.builder.start_node(SyntaxKind::CodeFenceClose.into());
            self.advance();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    fn parse_fenced_div(&mut self) {
        self.builder.start_node(SyntaxKind::FencedDiv.into());

        // Opening fence
        self.builder.start_node(SyntaxKind::DivFenceOpen.into());
        self.advance(); // div marker (:::)

        // Div info (class, attributes)
        if self.at(SyntaxKind::TEXT) || self.at(SyntaxKind::WHITESPACE) {
            self.builder.start_node(SyntaxKind::DivInfo.into());
            while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
                self.advance();
            }
            self.builder.finish_node();
        }

        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }
        self.builder.finish_node();

        // Div content
        self.builder.start_node(SyntaxKind::DivContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::DivMarker) {
            self.advance();
        }
        self.builder.finish_node();

        // Closing fence
        if self.at(SyntaxKind::DivMarker) {
            self.builder.start_node(SyntaxKind::DivFenceClose.into());
            self.advance();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    fn parse_math_block(&mut self) {
        self.builder.start_node(SyntaxKind::MathBlock.into());

        // Opening $$
        self.advance(); // consume opening $$

        // Content until closing $$
        while !self.at_eof() && !self.at(SyntaxKind::MathMarker) {
            self.advance();
        }

        // Closing $$
        if self.at(SyntaxKind::MathMarker) {
            self.advance();
        }

        self.builder.finish_node();
    }

    fn parse_paragraph(&mut self) {
        self.builder.start_node(SyntaxKind::PARAGRAPH.into());

        log::debug!("Starting paragraph parse at pos {}", self.pos);

        let mut iterations = 0;
        while !self.at_eof() {
            iterations += 1;
            if iterations > 1000 {
                panic!(
                    "Too many iterations in parse_paragraph! Current token: {:?} at pos {}",
                    self.current_token(),
                    self.pos
                );
            }

            let old_pos = self.pos;
            match self.current_token().map(|t| t.kind) {
                Some(SyntaxKind::NEWLINE) if self.is_blank_line() => {
                    log::trace!("Breaking paragraph on blank line");
                    break;
                }
                Some(SyntaxKind::FenceMarker | SyntaxKind::DivMarker | SyntaxKind::MathMarker) => {
                    log::trace!("Breaking paragraph on fence/div/math marker");
                    break;
                }
                Some(SyntaxKind::WHITESPACE) => {
                    log::trace!("Paragraph iteration {iterations}: advancing whitespace");
                    self.advance();
                }
                Some(SyntaxKind::NEWLINE) => {
                    log::trace!("Paragraph iteration {iterations}: advancing newline");
                    self.advance();
                }
                Some(SyntaxKind::TEXT) => {
                    log::trace!(
                        "Paragraph iteration {iterations}: advancing text {:?}",
                        self.current_token()
                    );
                    self.advance();
                }
                _ => {
                    log::trace!(
                        "Paragraph iteration {iterations}: advancing other token {:?}",
                        self.current_token()
                    );
                    self.advance();
                }
            }

            if self.pos == old_pos {
                panic!(
                    "Parser stuck in paragraph! Not advancing from pos {} with token {:?}",
                    self.pos,
                    self.current_token()
                );
            }
        }

        log::debug!("Finished paragraph parse at pos {}", self.pos);
        self.builder.finish_node();
    }

    fn parse_blank_line(&mut self) {
        self.builder.start_node(SyntaxKind::BlankLine.into());
        while self.at(SyntaxKind::NEWLINE) || self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }
        self.builder.finish_node();
    }

    fn is_blank_line(&self) -> bool {
        // A blank line is a newline where the line contains only whitespace
        if !self.at(SyntaxKind::NEWLINE) {
            return false;
        }

        // Walk backwards from the current NEWLINE over any whitespace on this line.
        // If we hit another NEWLINE (or BOF) before any non-whitespace, it's a blank line.
        let mut i = self.pos;
        while i > 0 {
            let k = self.tokens[i - 1].kind;
            match k {
                SyntaxKind::WHITESPACE => {
                    i -= 1;
                }
                SyntaxKind::NEWLINE => {
                    // Previous token (ignoring whitespace) is a newline => blank line
                    return true;
                }
                _ => {
                    // Found non-whitespace content on this line => not blank
                    return false;
                }
            }
        }

        // Start of file before any content => leading blank line
        true
    }

    fn debug_tokens(&self) {
        log::debug!("Tokens:");
        for (i, token) in self.tokens.iter().enumerate() {
            let start = self.tokens[..i].iter().map(|t| t.len).sum::<usize>();
            let end = start + token.len;
            let text = &self.input[start..end];
            log::debug!("  {}: {:?} = {:?}", i, token.kind, text);
        }
    }
}

pub fn parse(input: &str) -> SyntaxNode {
    Parser::new(input).parse()
}
