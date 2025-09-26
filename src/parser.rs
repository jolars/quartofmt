use crate::lexer::{Token, tokenize};
use crate::syntax::{SyntaxKind, SyntaxNode};
use rowan::GreenNodeBuilder;

pub fn token_offset(tokens: &[Token], idx: usize) -> usize {
    tokens[..idx].iter().map(|t| t.len).sum()
}

pub struct Parser<'a> {
    input: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    byte_offset: usize,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let tokens = tokenize(input);
        Self {
            input,
            tokens,
            pos: 0,
            byte_offset: 0,
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

    fn token_text(&self, idx: usize) -> &str {
        let start = self.tokens[..idx].iter().map(|t| t.len).sum::<usize>();
        let end = start + self.tokens[idx].len;
        &self.input[start..end]
    }

    fn advance(&mut self) {
        let token_opt = self.current_token().cloned();
        if let Some(token) = token_opt {
            let text = &self.input[self.byte_offset..self.byte_offset + token.len];
            log::trace!("Advancing: {:?} = {:?}", token.kind, text);
            self.builder.token(token.kind.into(), text);
            self.byte_offset += token.len;
            self.pos += 1;
        } else {
            log::trace!("Advance called but no current token");
        }
    }

    // Advance one token without emitting it into the CST
    fn skip_token(&mut self) {
        if let Some(token) = self.current_token().cloned() {
            self.byte_offset += token.len;
            self.pos += 1;
        }
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current_token()
            .map(|token| token.kind == kind)
            .unwrap_or(false)
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn at_eol(&self) -> bool {
        self.at(SyntaxKind::NEWLINE) || self.at(SyntaxKind::BlankLine)
    }

    fn at_eol_or_eof(&self) -> bool {
        self.at_eol() || self.at_eof()
    }

    fn at_bol(&self) -> bool {
        if self.pos == 0 {
            return true;
        }
        // previous non-whitespace token is NEWLINE
        let mut i = self.pos;
        while i > 0 && self.tokens[i - 1].kind == SyntaxKind::WHITESPACE {
            i -= 1;
        }

        if i == 0 {
            true
        } else {
            self.tokens[i - 1].kind == SyntaxKind::NEWLINE
                || self.tokens[i - 1].kind == SyntaxKind::BlankLine
        }
    }

    fn parse_document(&mut self) {
        self.builder.start_node(SyntaxKind::DOCUMENT.into());

        log::debug!("Starting document parse");

        #[cfg(debug_assertions)]
        {
            self.debug_tokens();
        }

        // Parse the rest of the top-level blocks until EOF
        self.parse_blocks(|_| false);

        self.builder.finish_node();
    }

    fn parse_blocks<F>(&mut self, stop: F)
    where
        F: Fn(&Parser) -> bool,
    {
        let mut iterations = 0;
        while !self.at_eof() && !stop(self) {
            iterations += 1;
            #[cfg(debug_assertions)]
            {
                if iterations > 1000 {
                    panic!(
                        "Too many iterations in parse_blocks! Current token: {:?} at pos {pos}",
                        self.current_token(),
                        pos = self.pos
                    );
                }
            }

            log::trace!(
                "parse_blocks iteration {iterations}: pos={}, token={:?}",
                self.pos,
                self.current_token()
            );

            let old_pos = self.pos;

            // Headings first
            if self.is_atx_heading_start() {
                self.parse_atx_heading();
                continue;
            }

            if self.is_setext_heading_start() {
                self.parse_setext_heading();
                continue;
            }

            // Tables
            if self.is_simple_table_start() {
                self.parse_simple_table();
                continue;
            }

            match self.current_token().map(|t| t.kind) {
                Some(SyntaxKind::FrontmatterDelim) => self.parse_frontmatter(),
                Some(SyntaxKind::CodeFenceMarker) => self.parse_code_block(),
                Some(SyntaxKind::DivMarker) => self.parse_fenced_div(),
                Some(SyntaxKind::BlockMathMarker) => self.parse_block_math(),
                Some(SyntaxKind::CommentStart) => self.parse_comment(),
                Some(SyntaxKind::LatexCommand) if self.is_standalone_latex_command() => {
                    log::debug!("Parsing standalone LaTeX command");
                    self.parse_standalone_latex_command()
                }
                Some(SyntaxKind::LatexEnvBegin) => self.parse_latex_environment(),
                Some(SyntaxKind::BlockQuoteMarker) => self.parse_block_quote(),
                Some(SyntaxKind::ListMarker) => self.parse_list(0),
                Some(SyntaxKind::BlankLine) => self.parse_blank_line(),
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
                    "Parser stuck in parse_blocks! Not advancing from pos {} with token {:?}",
                    self.pos,
                    self.current_token()
                );
            }
        }
    }

    fn parse_frontmatter(&mut self) {
        self.builder.start_node(SyntaxKind::FRONTMATTER.into());

        log::debug!("Starting frontmatter parse at pos {}", self.pos);

        self.advance(); // ---/+++

        // Skip to end of line after opening delimiter
        while !self.at_eol_or_eof() {
            self.advance();
        }

        if self.at_eol() {
            self.advance();
        }

        // Content until closing delimiter
        while !self.at_eof() && !self.at(SyntaxKind::FrontmatterDelim) {
            self.advance();
        }

        if self.at_eof() {
            panic!("Could not find closing frontmatter delimiter");
        }

        // Closing delimiter
        if self.at(SyntaxKind::FrontmatterDelim) {
            log::debug!("Found closing delimiter");
            self.advance();
            // Skip to end of line after closing delimiter
            while !self.at_eol_or_eof() {
                self.advance();
            }

            if self.at(SyntaxKind::NEWLINE) {
                self.advance();
            }
        }

        log::debug!("Finished frontmatter parse at pos {}", self.pos);
        self.builder.finish_node();
    }

    fn parse_inline_footnote(&mut self) {
        self.builder.start_node(SyntaxKind::InlineFootnote.into());
        self.advance(); // consume ^[
        // Content until closing ]
        while !self.at_eof() && !self.at(SyntaxKind::InlineFootnoteEnd) {
            self.advance();
        }
        if self.at(SyntaxKind::InlineFootnoteEnd) {
            self.advance(); // consume ]
        }
        self.builder.finish_node();
        log::debug!("Finished inline footnote at pos {}", self.pos);
        log::trace!(
            "Current token after inline footnote: {:?}",
            self.current_token()
        );
    }

    fn parse_code_block(&mut self) {
        self.builder.start_node(SyntaxKind::CodeBlock.into());

        // Opening fence
        self.builder.start_node(SyntaxKind::CodeFenceOpen.into());
        self.advance(); // fence marker

        // Code info (language, options)
        if self.at(SyntaxKind::TEXT) || self.at(SyntaxKind::WHITESPACE) {
            self.builder.start_node(SyntaxKind::CodeInfo.into());
            while !self.at_eof() && !self.at_eol() {
                self.advance();
            }
            self.builder.finish_node();
        }

        if self.at_eol() {
            self.advance();
        }

        self.builder.finish_node();

        // Code content
        self.builder.start_node(SyntaxKind::CodeContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::CodeFenceMarker) {
            self.advance();
        }
        self.builder.finish_node();

        // Closing fence
        if self.at(SyntaxKind::CodeFenceMarker) {
            self.builder.start_node(SyntaxKind::CodeFenceClose.into());
            self.advance();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    fn parse_latex_environment(&mut self) {
        self.builder.start_node(SyntaxKind::LatexEnvironment.into());

        // Begin token
        if self.at(SyntaxKind::LatexEnvBegin) {
            self.builder.start_node(SyntaxKind::LatexEnvBegin.into());
            self.advance();
            self.builder.finish_node();
        }

        // Content until matching end
        self.builder.start_node(SyntaxKind::LatexEnvContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::LatexEnvEnd) {
            self.advance();
        }
        self.builder.finish_node();

        // End token
        if self.at(SyntaxKind::LatexEnvEnd) {
            self.builder.start_node(SyntaxKind::LatexEnvEnd.into());
            self.advance();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    fn parse_fenced_div(&mut self) {
        log::debug!("Starting parse_fenced_div at pos {}", self.pos);

        self.builder.start_node(SyntaxKind::FencedDiv.into());

        let mut open_len = 0;
        self.builder.start_node(SyntaxKind::DivFenceOpen.into());
        if let Some(token) = self.current_token() {
            open_len = token.len;
        }
        log::debug!(
            "Starting DivFenceOpen (len {}), current token: {:?}",
            open_len,
            self.current_token()
        );
        self.advance(); // consume opening DivMarker               

        // Div info (class, attributes) - capture whitespace and text on same line
        while !self.at_eof() && !self.at_eol() {
            log::debug!("Adding to DivFenceOpen: {:?}", self.current_token());
            self.advance();
        }

        if self.at_eol() {
            log::debug!("Adding newline to DivFenceOpen: {:?}", self.current_token());
            self.advance();
        }
        self.builder.finish_node();
        log::debug!("Finished DivFenceOpen");

        // Div content (include nested divs until matching fence length)
        self.builder.start_node(SyntaxKind::DivContent.into());
        self.parse_blocks(|p| {
            if p.at(SyntaxKind::DivMarker)
                && let Some(tok) = p.current_token()
            {
                return tok.len == open_len;
            }
            false
        });
        self.builder.finish_node();

        // Closing fence matching opening length
        if self.at(SyntaxKind::DivMarker)
            && self.current_token().is_some_and(|tok| tok.len == open_len)
        {
            self.builder.start_node(SyntaxKind::DivFenceClose.into());
            self.advance();
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }

    fn parse_comment(&mut self) {
        self.builder.start_node(SyntaxKind::Comment.into());

        // Opening <!--
        if self.at(SyntaxKind::CommentStart) {
            self.advance();
        } else {
            panic!("Expected CommentStart at beginning of parse_comment");
        }

        // Content until closing -->
        while !self.at_eof() && !self.at(SyntaxKind::CommentEnd) {
            self.advance();
        }

        // Closing -->
        if self.at(SyntaxKind::CommentEnd) {
            self.advance();
        }

        self.builder.finish_node();
    }

    fn parse_link(&mut self) {
        self.builder.start_node(SyntaxKind::Link.into());
        self.advance(); // Link token
        // Attach attribute if present
        if self.at(SyntaxKind::Attribute) {
            self.builder.start_node(SyntaxKind::Attribute.into());
            self.advance();
            self.builder.finish_node();
        }
        self.builder.finish_node();
    }

    fn parse_image_link(&mut self) {
        self.builder.start_node(SyntaxKind::ImageLink.into());
        self.advance(); // ImageLink token
        // Attach attribute if present
        if self.at(SyntaxKind::Attribute) {
            self.builder.start_node(SyntaxKind::Attribute.into());
            self.advance();
            self.builder.finish_node();
        }
        self.builder.finish_node();
    }

    fn parse_inline_math(&mut self) {
        self.builder.start_node(SyntaxKind::InlineMath.into());
        // Opening $
        if self.at(SyntaxKind::InlineMathMarker) {
            self.builder.start_node(SyntaxKind::InlineMathMarker.into());
            self.advance();
            self.builder.finish_node();
        }
        // Content until closing $
        self.builder.start_node(SyntaxKind::MathContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::InlineMathMarker) {
            self.advance();
        }
        self.builder.finish_node();
        // Closing $
        if self.at(SyntaxKind::InlineMathMarker) {
            self.builder.start_node(SyntaxKind::InlineMathMarker.into());
            self.advance();
            self.builder.finish_node();
        }
        self.builder.finish_node();
    }

    fn parse_block_math(&mut self) {
        self.builder.start_node(SyntaxKind::MathBlock.into());

        // Opening $$
        if self.at(SyntaxKind::BlockMathMarker) {
            self.builder.start_node(SyntaxKind::BlockMathMarker.into());
            self.advance();
            self.builder.finish_node();
        }

        // Math content (until closing $$)
        self.builder.start_node(SyntaxKind::MathContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::BlockMathMarker) {
            self.advance();
        }
        self.builder.finish_node();

        // Closing $$
        if self.at(SyntaxKind::BlockMathMarker) {
            self.builder.start_node(SyntaxKind::BlockMathMarker.into());
            self.advance();
            self.builder.finish_node();
        }

        // Optional whitespace
        if self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }

        // Optional label
        if self.at(SyntaxKind::Attribute) {
            self.builder.start_node(SyntaxKind::Attribute.into());
            self.advance();
            self.builder.finish_node();
        }

        // Trailing newline
        if self.at_eol() {
            self.advance();
        }

        self.builder.finish_node();
    }

    fn parse_block_quote(&mut self) {
        self.builder.start_node(SyntaxKind::BlockQuote.into());

        // Track whether the previous emitted child was a quoted blank line.
        // Pandoc requires a blank line before starting a nested block quote.
        let mut last_was_blank = false;

        // Start with the initial quoted line
        let mut first = true;
        while !self.at_eof() {
            // Only require BlockQuoteMarker for the first line of a block quote paragraph
            if first {
                if !self.at(SyntaxKind::BlockQuoteMarker) {
                    break;
                }
                self.advance(); // '>'
                if self.at(SyntaxKind::WHITESPACE) {
                    self.advance();
                }

                // Quoted blank line: emit a BlankLine node
                if self.at(SyntaxKind::NEWLINE) {
                    self.builder.start_node(SyntaxKind::BlankLine.into());
                    self.advance(); // consume newline
                    self.builder.finish_node();
                    // Next line may start a new block quote paragraph
                    first = true;
                    last_was_blank = true;
                    continue;
                }

                // If there was a quoted blank line before, allow nested block quote:
                // Detect an immediate second '>' to start a nested quote (Pandoc behavior).
                if last_was_blank && self.at(SyntaxKind::BlockQuoteMarker) {
                    // Recurse into nested block quote starting at this marker.
                    self.parse_block_quote();
                    // After nested block, reset state to look for next sibling in this quote
                    first = true;
                    last_was_blank = false;
                    continue;
                }
            }

            // Start a single paragraph that may span multiple quoted and lazy lines
            self.builder.start_node(SyntaxKind::PARAGRAPH.into());

            loop {
                // Consume content until end of line
                while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
                    self.advance();
                }
                // Include the newline inside the paragraph so it becomes space when formatted
                if self.at(SyntaxKind::NEWLINE) {
                    self.advance();
                }

                if self.at(SyntaxKind::BlockQuoteMarker) {
                    // Look ahead without emitting into the paragraph
                    let mut tmp = self.pos + 1; // after '>'
                    // Skip optional whitespace
                    while tmp < self.tokens.len() && self.tokens[tmp].kind == SyntaxKind::WHITESPACE
                    {
                        tmp += 1;
                    }
                    // If immediately a newline, this is a quoted blank line -> finish this paragraph
                    if tmp < self.tokens.len() && self.tokens[tmp].kind == SyntaxKind::NEWLINE {
                        break;
                    }

                    // Otherwise, it's a continued quoted line. Strip all leading '>' markers
                    // (and the whitespace after each) so they are NOT included in paragraph text.
                    while self.at(SyntaxKind::BlockQuoteMarker) {
                        self.skip_token(); // skip '>'
                        if self.at(SyntaxKind::WHITESPACE) {
                            self.skip_token();
                        }
                    }
                    continue;
                }

                // If the next line is not quoted but is not blank, treat as lazy continuation
                // (i.e., part of the same block quote paragraph)
                if self.at(SyntaxKind::TEXT) || self.at(SyntaxKind::WHITESPACE) {
                    continue;
                }

                // Next line is blank or a block boundary: finish this paragraph
                break;
            }

            self.builder.finish_node();
            // After a paragraph, expect either a blank line or a new block quote marker
            first = true;
            last_was_blank = false;
        }

        self.builder.finish_node();
    }

    fn parse_list(&mut self, indent: usize) {
        self.builder.start_node(SyntaxKind::List.into());

        while !self.at_eof() {
            // Count leading whitespace to determine indentation level
            let mut current_indent = 0;
            let mut temp_pos = self.pos;

            // Skip any leading whitespace and count it
            while temp_pos < self.tokens.len()
                && self.tokens[temp_pos].kind == SyntaxKind::WHITESPACE
            {
                current_indent += self.tokens[temp_pos].len;
                temp_pos += 1;
            }

            // Check if we have a list marker at the expected indentation
            if temp_pos < self.tokens.len() && self.tokens[temp_pos].kind == SyntaxKind::ListMarker
            {
                if current_indent == indent {
                    // Same level - parse as list item
                    self.parse_list_item();
                } else if current_indent > indent {
                    // Deeper level - parse as nested list
                    self.parse_list(current_indent);
                } else {
                    // Shallower level - end this list
                    break;
                }
            } else {
                // No list marker found, end the list
                break;
            }
        }

        self.builder.finish_node();
    }

    fn parse_list_item(&mut self) {
        self.builder.start_node(SyntaxKind::ListItem.into());

        // Consume leading whitespace (indentation)
        while self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }

        // Consume the list marker (-, +, *)
        if self.at(SyntaxKind::ListMarker) {
            self.advance();
        }

        // Consume the space after the marker
        if self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }

        // Parse the content of this list item until we hit a blank line or another list marker
        while !self.at_eof() {
            // Check if we've hit a blank line or another list item at the same or shallower level
            if self.at(SyntaxKind::NEWLINE) && self.is_blank_line() {
                break;
            }

            // Look ahead to see if the next non-whitespace token after a newline is a list marker
            if self.at(SyntaxKind::NEWLINE) {
                let mut temp_pos = self.pos + 1;

                // Skip whitespace after the newline
                while temp_pos < self.tokens.len()
                    && self.tokens[temp_pos].kind == SyntaxKind::WHITESPACE
                {
                    temp_pos += 1;
                }

                // If we find a list marker, this list item is done
                if temp_pos < self.tokens.len()
                    && self.tokens[temp_pos].kind == SyntaxKind::ListMarker
                {
                    // Consume the current newline and stop
                    self.advance();
                    break;
                }
            }

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
                Some(SyntaxKind::BlankLine) => {
                    log::trace!("Breaking paragraph on blank line");
                    break;
                }

                Some(SyntaxKind::CommentStart) => {
                    // End paragraph, parse comment separately
                    break;
                }

                Some(SyntaxKind::Link) | Some(SyntaxKind::ImageLink) => {
                    log::trace!("Paragraph: parsing link or image link");
                    match self.current_token().map(|t| t.kind) {
                        Some(SyntaxKind::Link) => self.parse_link(),
                        Some(SyntaxKind::ImageLink) => self.parse_image_link(),
                        _ => unreachable!(),
                    }
                }

                Some(
                    SyntaxKind::CodeFenceMarker
                    | SyntaxKind::DivMarker
                    | SyntaxKind::BlockMathMarker,
                ) => {
                    log::trace!("Breaking paragraph on fence/div/math marker");
                    break;
                }

                // Keep inline math inside the paragraph
                Some(SyntaxKind::InlineMathMarker) => {
                    log::trace!("Paragraph: parsing inline math");
                    self.parse_inline_math();
                }

                Some(SyntaxKind::InlineFootnoteStart) => {
                    log::trace!("Paragraph: parsing inline footnote");
                    self.parse_inline_footnote();
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

                Some(SyntaxKind::LatexCommand) => {
                    log::debug!("LaTeX command in paragraph");
                    log::trace!(
                        "Paragraph iteration {iterations}: advancing latex command {:?}",
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
        while self.at(SyntaxKind::NEWLINE)
            || self.at(SyntaxKind::BlankLine)
            || self.at(SyntaxKind::WHITESPACE)
        {
            self.advance();
        }
        self.builder.finish_node();
    }

    fn is_simple_table_start(&self) -> bool {
        // Only consider table starts at the beginning of a line
        if !self.at_bol() {
            return false;
        }

        // Try headered simple table:
        // 1) header line (TEXT/WHITESPACE), 2) dashed line, 3) at least one body row
        {
            let mut pos = self.pos;

            // 1. First line: TEXT or WHITESPACE, then NEWLINE
            let mut saw_text = false;
            while pos < self.tokens.len() {
                match self.tokens[pos].kind {
                    SyntaxKind::TEXT => {
                        // First line must have some non-dash content; a dashed-only line
                        // is not a header for the headered table form.
                        let start = token_offset(&self.tokens, pos);
                        let end = start + self.tokens[pos].len;
                        let text = &self.input[start..end];
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && !trimmed.chars().all(|c| c == '-' || c == ' ') {
                            saw_text = true;
                        }
                        pos += 1;
                    }

                    SyntaxKind::WHITESPACE => pos += 1,

                    SyntaxKind::NEWLINE => {
                        pos += 1;
                        break;
                    }

                    _ => break,
                }
            }

            if saw_text {
                // 2. Second line: dashes (one or more TEXT tokens containing only '-' and spaces), then NEWLINE
                let mut saw_dash = false;
                while pos < self.tokens.len() {
                    match self.tokens[pos].kind {
                        SyntaxKind::TEXT => {
                            let start = token_offset(&self.tokens, pos);
                            let end = start + self.tokens[pos].len;
                            let text = &self.input[start..end];
                            log::debug!("SimpleTable candidate line {pos}: {text}");
                            let trimmed = text.trim();

                            if !trimmed.is_empty()
                                && trimmed.contains('-')
                                && trimmed.chars().all(|c| c == '-' || c == ' ')
                            {
                                saw_dash = true;
                                pos += 1;
                            } else {
                                // Not a dashed line; abort headered form
                                saw_dash = false;
                                break;
                            }
                        }

                        SyntaxKind::WHITESPACE => {
                            pos += 1;
                        }

                        SyntaxKind::NEWLINE => {
                            pos += 1;
                            break;
                        }

                        _ => break,
                    }
                }

                if saw_dash {
                    // 3. Third line: TEXT or WHITESPACE (at least one row), then NEWLINE
                    let mut saw_row = false;
                    while pos < self.tokens.len() {
                        match self.tokens[pos].kind {
                            SyntaxKind::TEXT | SyntaxKind::WHITESPACE => {
                                saw_row = true;
                                pos += 1;
                            }

                            SyntaxKind::NEWLINE => {
                                break;
                            }

                            _ => break,
                        }
                    }

                    if saw_row {
                        return true;
                    }
                }
            }
        }

        // Try headerless simple table:
        // 1) first line is dashed separator
        // 2) at least one body row line follows
        {
            let mut pos = self.pos;

            // 1. Dashed line (one or more TEXT tokens of '-' and spaces), allow interspersed WHITESPACE tokens
            let mut saw_dash = false;
            while pos < self.tokens.len() {
                match self.tokens[pos].kind {
                    SyntaxKind::TEXT => {
                        let start = token_offset(&self.tokens, pos);
                        let end = start + self.tokens[pos].len;
                        let text = &self.input[start..end];
                        if text.trim().chars().all(|c| c == '-' || c == ' ') {
                            saw_dash = true;
                            pos += 1;
                        } else {
                            break;
                        }
                    }

                    SyntaxKind::WHITESPACE => {
                        pos += 1;
                    }

                    SyntaxKind::NEWLINE => {
                        pos += 1;
                        break;
                    }

                    _ => break,
                }
            }

            if saw_dash {
                log::debug!("Headerless SimpleTable dashed line found");

                // 2. At least one body row line (TEXT/WHITESPACE until NEWLINE)
                let mut saw_row = false;
                while pos < self.tokens.len() {
                    match self.tokens[pos].kind {
                        SyntaxKind::TEXT | SyntaxKind::WHITESPACE => {
                            saw_row = true;
                            pos += 1;
                        }

                        SyntaxKind::NEWLINE => {
                            break;
                        }

                        _ => break,
                    }
                }

                if !saw_row {
                    log::debug!("Headerless SimpleTable: no body row found after dashed line");
                } else {
                    log::debug!("Headerless SimpleTable body row found");

                    // 3. Require a closing dashed line
                    if pos < self.tokens.len()
                        && (self.tokens[pos].kind == SyntaxKind::BlankLine
                            || self.tokens[pos].kind == SyntaxKind::NEWLINE)
                    {
                        pos += 1;
                    }

                    let mut saw_closing_dash = false;
                    while pos < self.tokens.len() {
                        match self.tokens[pos].kind {
                            SyntaxKind::TEXT => {
                                let start = token_offset(&self.tokens, pos);
                                let end = start + self.tokens[pos].len;
                                let text = &self.input[start..end];
                                let trimmed = text.trim();
                                if !trimmed.is_empty()
                                    && trimmed.contains('-')
                                    && trimmed.chars().all(|c| c == '-' || c == ' ')
                                {
                                    saw_closing_dash = true;
                                    pos += 1;
                                } else {
                                    // Not a dashed line -> not a headerless simple table
                                    break;
                                }
                            }
                            SyntaxKind::WHITESPACE => {
                                pos += 1;
                            }

                            SyntaxKind::NEWLINE => {
                                // end of closing dashed line
                                break;
                            }

                            _ => {
                                log::debug!(
                                    "Headerless SimpleTable: unexpected token after body row: {:?}",
                                    self.tokens[pos]
                                );
                                break;
                            }
                        }
                    }

                    log::debug!(
                        "Headerless SimpleTable closing dashed line found: {saw_closing_dash}"
                    );

                    if saw_closing_dash {
                        return true;
                    }
                }
            }
        }

        // Return true if either table format was detected
        false
    }

    fn parse_simple_table(&mut self) {
        self.builder.start_node(SyntaxKind::SimpleTable.into());
        // Parse lines until we hit a blank line or a non-table line
        let mut _line_count = 0;
        while !self.at_eof() {
            // Stop on blank line or if not TEXT/WHITESPACE/NEWLINE
            let mut temp_pos = self.pos;
            let mut saw_content = false;
            while temp_pos < self.tokens.len() {
                match self.tokens[temp_pos].kind {
                    SyntaxKind::TEXT | SyntaxKind::WHITESPACE => {
                        saw_content = true;
                        temp_pos += 1;
                    }

                    SyntaxKind::NEWLINE => {
                        temp_pos += 1;
                        break;
                    }

                    _ => break,
                }
            }
            if !saw_content {
                break;
            }
            // Consume this line, logging each token's text
            while self.pos < temp_pos {
                let start = token_offset(&self.tokens, self.pos);
                let end = start + self.tokens[self.pos].len;
                let text = &self.input[start..end];
                log::debug!("SimpleTable line token {}: {:?}", self.pos, text);
                self.advance();
            }
            // Explicitly consume the trailing NEWLINE for each line, if present
            if self.at_eol() {
                self.advance();
            }
            // _line_count += 1;
        }
        self.builder.finish_node();
    }

    fn parse_standalone_latex_command(&mut self) {
        // Consume any leading whitespace (indentation)
        while self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }

        // Parse the LaTeX command including its trailing newline as one unit
        self.builder.start_node(SyntaxKind::LatexCommand.into());
        self.advance(); // consume the latex command token
        if self.at(SyntaxKind::NEWLINE) {
            self.advance(); // include the trailing newline in the command node
        }
        self.builder.finish_node();
    }

    fn is_standalone_latex_command(&self) -> bool {
        // Check if the current LaTeX command is on its own line
        if !self.at(SyntaxKind::LatexCommand) {
            return false;
        }

        // Look backwards to see if there's any non-whitespace content before this on the line
        let mut i = self.pos;
        while i > 0 {
            match self.tokens[i - 1].kind {
                SyntaxKind::WHITESPACE => i -= 1,
                SyntaxKind::NEWLINE => break, // Found start of line
                _ => return false,            // Found other content on this line
            }
        }

        // Look forwards to see if there's any non-whitespace content after this on the line
        let mut j = self.pos + 1;
        while j < self.tokens.len() {
            match self.tokens[j].kind {
                SyntaxKind::WHITESPACE => j += 1,
                SyntaxKind::NEWLINE => break, // Found end of line
                _ => return false,            // Found other content on this line
            }
        }

        true
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

    fn is_atx_heading_start(&self) -> bool {
        if !self.at_bol() {
            return false;
        }
        let mut pos = self.pos;

        // Allow up to 3 leading spaces
        let mut leading_ws = 0;
        while pos < self.tokens.len() && self.tokens[pos].kind == SyntaxKind::WHITESPACE {
            leading_ws += self.tokens[pos].len;
            if leading_ws > 3 {
                return false;
            }
            pos += 1;
        }

        if pos >= self.tokens.len() || self.tokens[pos].kind != SyntaxKind::TEXT {
            return false;
        }
        let t = self.token_text(pos);
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if hashes == 0 || hashes > 6 || hashes != t.chars().count() {
            return false;
        }

        // Require a space after the opening hashes
        if pos + 1 >= self.tokens.len() || self.tokens[pos + 1].kind != SyntaxKind::WHITESPACE {
            return false;
        }

        true
    }

    fn is_setext_heading_start(&self) -> bool {
        if !self.at_bol() {
            return false;
        }
        // First line: some text (not empty)
        let mut pos = self.pos;
        let mut saw_text = false;
        while pos < self.tokens.len() {
            match self.tokens[pos].kind {
                SyntaxKind::NEWLINE => {
                    pos += 1;
                    break;
                }
                SyntaxKind::TEXT
                | SyntaxKind::WHITESPACE
                | SyntaxKind::LatexCommand
                | SyntaxKind::Link
                | SyntaxKind::ImageLink => {
                    // basic allowance of inline things
                    if self.tokens[pos].kind == SyntaxKind::TEXT {
                        // Any non-empty text counts
                        if !self.token_text(pos).trim().is_empty() {
                            saw_text = true;
                        }
                    }
                    pos += 1;
                }
                _ => return false,
            }
        }
        if !saw_text {
            return false;
        }

        // Second line: TEXT token of only '=' or '-' (no spaces inside), then optional whitespace, then NEWLINE
        if pos >= self.tokens.len() || self.tokens[pos].kind != SyntaxKind::TEXT {
            return false;
        }
        let underline = self.token_text(pos);
        if underline.is_empty() || !underline.chars().all(|c| c == '=' || c == '-') {
            return false;
        }
        // Determine level
        let level = if underline.starts_with('=') { 1 } else { 2 };
        // Mixed chars not allowed
        if (level == 1 && underline.chars().any(|c| c != '='))
            || (level == 2 && underline.chars().any(|c| c != '-'))
        {
            return false;
        }
        // Next must be NEWLINE or WHITESPACE then NEWLINE
        let mut p2 = pos + 1;
        if p2 < self.tokens.len() && self.tokens[p2].kind == SyntaxKind::WHITESPACE {
            p2 += 1;
        }
        if p2 >= self.tokens.len() || self.tokens[p2].kind != SyntaxKind::NEWLINE {
            return false;
        }

        true
    }

    fn parse_atx_heading(&mut self) {
        self.builder.start_node(SyntaxKind::Heading.into());

        // Optional leading spaces (up to 3)
        while self.at(SyntaxKind::WHITESPACE) && self.current_token().unwrap().len <= 3 {
            self.advance();
        }

        // Marker
        if self.at(SyntaxKind::TEXT) {
            self.builder.start_node(SyntaxKind::AtxHeadingMarker.into());
            self.advance();
            self.builder.finish_node();
        }
        // One space
        if self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }

        // Content until end of line (we'll later ignore trailing closing hashes in formatter)
        self.builder.start_node(SyntaxKind::HeadingContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }
        self.builder.finish_node();

        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }

        self.builder.finish_node();
    }

    fn parse_setext_heading(&mut self) {
        self.builder.start_node(SyntaxKind::Heading.into());

        // First line: content
        self.builder.start_node(SyntaxKind::HeadingContent.into());
        while !self.at_eof() && !self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }
        self.builder.finish_node();

        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }

        // Second line: underline marker
        if self.at(SyntaxKind::TEXT) {
            self.builder
                .start_node(SyntaxKind::SetextHeadingUnderline.into());
            self.advance();
            self.builder.finish_node();
        }
        // Optional trailing whitespace
        if self.at(SyntaxKind::WHITESPACE) {
            self.advance();
        }
        // End of underline line
        if self.at(SyntaxKind::NEWLINE) {
            self.advance();
        }

        self.builder.finish_node();
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

#[cfg(test)]
mod tests;
