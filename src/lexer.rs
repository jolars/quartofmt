use crate::syntax::SyntaxKind;

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: usize,
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    pub fn peek_char(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    pub fn at_eol(&self) -> bool {
        self.current_char() == Some('\n') || self.pos >= self.input.len()
    }

    pub fn at_bol_with_indent(&self) -> Option<usize> {
        // Returns Some(indent) if at BOL and indent <= 3, else None
        if self.pos == 0 {
            return Some(0);
        }
        let pos = self.pos;
        let line_start = self.input[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prefix = &self.input[line_start..pos];
        let mut indent = 0;
        for ch in prefix.chars() {
            match ch {
                ' ' => indent += 1,
                '\t' => indent += 4,
                _ => return None, // Non-whitespace before marker: not BOL
            }
        }
        if indent <= 3 { Some(indent) } else { None }
    }

    pub fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn is_list_marker(&self) -> bool {
        // Only treat -, +, * as list marker at BOL or after newline/indent-only whitespace
        let pos = self.pos;
        // Find start of line
        let line_start = self.input[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prefix = &self.input[line_start..pos];
        // Only allow whitespace before the marker
        let is_bol_or_indent = prefix.chars().all(|c| c == ' ' || c == '\t');
        if is_bol_or_indent
            && let Some(ch) = self.current_char()
            && matches!(ch, '-' | '+' | '*')
        {
            return self.peek_char(1) == Some(' ');
        }
        false
    }

    fn is_numbered_list_marker(&self) -> bool {
        // Check if this is a numbered list marker (digit(s) followed by . and space)
        let mut offset = 0;

        // Must start with a digit
        if let Some(ch) = self.peek_char(offset) {
            if !ch.is_ascii_digit() {
                return false;
            }
        } else {
            return false;
        }

        // Continue while we have digits
        while let Some(ch) = self.peek_char(offset) {
            if ch.is_ascii_digit() {
                offset += 1;
            } else {
                break;
            }
        }

        // Must be followed by . and space
        self.peek_char(offset) == Some('.') && self.peek_char(offset + 1) == Some(' ')
    }

    fn is_block_quote_marker(&self) -> bool {
        if self.current_char() != Some('>') {
            return false;
        }

        let pos = self.pos;
        // Start of current line
        let line_start = self.input[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prefix = &self.input[line_start..pos];

        // Count indentation (spaces/tabs) before the first non-ws char on the line
        let mut indent_cols = 0usize;
        let mut idx_bytes = 0usize;
        for ch in prefix.chars() {
            match ch {
                ' ' => {
                    indent_cols += 1;
                    idx_bytes += 1;
                }
                '\t' => {
                    indent_cols += 4;
                    idx_bytes += 1;
                }
                _ => break,
            }
        }
        if indent_cols > 3 {
            return false;
        }

        // After indentation, allow only sequences of '>' and spaces before this '>'
        // This permits nested markers on the same line like "> > Text" or ">> Text".
        let rest = &prefix[idx_bytes..];
        if !rest.chars().all(|c| c == '>' || c == ' ') {
            // Some other non-whitespace content before '>' on this line
            return false;
        }

        // Count how many '>' markers already appeared on this line before current pos.
        let prior_markers = rest.chars().filter(|&c| c == '>').count();

        // Enforce Pandoc's blank_before_blockquote only for the first '>' on the line.
        if prior_markers == 0 {
            // Require blank line before block quote (Pandoc default), unless at BOF
            if line_start == 0 {
                return true;
            }
            let prev_line_end = line_start - 1;
            let prev_line_start = self.input[..prev_line_end]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let prev_line = &self.input[prev_line_start..prev_line_end];

            // If previous line is blank, ok
            if prev_line
                .trim_matches(|c| c == ' ' || c == '\t' || c == '\r')
                .is_empty()
            {
                return true;
            }

            // Otherwise, allow continuation if previous line is itself a block quote line
            let mut chars = prev_line.chars();
            let mut prev_indent = 0usize;
            loop {
                match chars.clone().next() {
                    Some(' ') => {
                        prev_indent += 1;
                        chars.next();
                    }
                    Some('\t') => {
                        prev_indent += 4;
                        chars.next();
                    }
                    _ => break,
                }
            }
            return prev_indent <= 3 && chars.next() == Some('>');
        }

        // Not the first '>' on the line (nested marker) -> allowed.
        true
    }

    pub fn advance_while<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        let mut iterations = 0;
        while let Some(ch) = self.current_char() {
            iterations += 1;
            debug_assert!(
                iterations <= 1000,
                "Infinite loop in advance_while! pos: {}, char: {:?}",
                self.pos,
                ch
            );
            if predicate(ch) {
                self.advance();
            } else {
                break;
            }
        }
        self.pos - start
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.input[self.pos..].starts_with(prefix)
    }

    pub fn next_token(&mut self) -> Option<Token> {
        if self.pos >= self.input.len() {
            return None;
        }

        let ch = self.current_char()?;

        let indent = self.at_bol_with_indent();

        if let Some(_indent) = indent {
            // At BOL with up to 3 spaces/tabs of indent
            match ch {
                // Code fence (``` or ~~~)
                '`' | '~' if self.starts_with("```") || self.starts_with("~~~") => {
                    let tick_count = self.advance_while(|c| c == '`' || c == '~');
                    if tick_count >= 3 {
                        return Some(Token {
                            kind: SyntaxKind::CodeFenceMarker,
                            len: tick_count,
                        });
                    }
                }

                ':' if self.starts_with(":::") => {
                    let len = self.advance_while(|c| c == ':');
                    return Some(Token {
                        kind: SyntaxKind::DivMarker,
                        len,
                    });
                }

                _ => { /* continue processing below */ }
            }
        }

        // Detect LaTeX environment begin: \begin{...}
        if self.starts_with("\\begin{") {
            let start_pos = self.pos;
            // Consume \begin{
            for _ in 0.."\\begin{".len() {
                self.advance();
            }
            // Consume until }
            while let Some(c) = self.current_char() {
                self.advance();
                if c == '}' {
                    break;
                }
            }
            let len = self.pos - start_pos;
            return Some(Token {
                kind: SyntaxKind::LatexEnvBegin,
                len,
            });
        }

        // Detect LaTeX environment end: \end{...}
        if self.starts_with("\\end{") {
            let start_pos = self.pos;
            // Consume \end{
            for _ in 0.."\\end{".len() {
                self.advance();
            }
            // Consume until }
            while let Some(c) = self.current_char() {
                self.advance();
                if c == '}' {
                    break;
                }
            }
            let len = self.pos - start_pos;
            return Some(Token {
                kind: SyntaxKind::LatexEnvEnd,
                len,
            });
        }

        match ch {
            '\n' => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::NEWLINE,
                    len: 1,
                })
            }

            ' ' | '\t' | '\r' => {
                let len = self.advance_while(|c| matches!(c, ' ' | '\t' | '\r'));
                Some(Token {
                    kind: SyntaxKind::WHITESPACE,
                    len,
                })
            }

            '\\' => {
                // Check if this is a LaTeX command
                if let Some(next_ch) = self.peek_char(1)
                    && next_ch.is_ascii_alphabetic()
                {
                    // This is a LaTeX command like \command
                    let start_pos = self.pos;
                    self.advance(); // consume \

                    // Consume command name (alphabetic characters)
                    while let Some(ch) = self.current_char() {
                        if ch.is_ascii_alphabetic() {
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    // Consume optional square bracket arguments [...]
                    while let Some(ch) = self.current_char() {
                        if ch == '[' {
                            let mut bracket_count = 0;
                            while let Some(ch) = self.current_char() {
                                self.advance();
                                if ch == '[' {
                                    bracket_count += 1;
                                } else if ch == ']' {
                                    bracket_count -= 1;
                                    if bracket_count == 0 {
                                        break;
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    // Consume any arguments in braces {...}
                    while let Some(ch) = self.current_char() {
                        if ch == '{' {
                            let mut brace_count = 0;
                            while let Some(ch) = self.current_char() {
                                self.advance();
                                if ch == '{' {
                                    brace_count += 1;
                                } else if ch == '}' {
                                    brace_count -= 1;
                                    if brace_count == 0 {
                                        break;
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    let len = self.pos - start_pos;
                    return Some(Token {
                        kind: SyntaxKind::LatexCommand,
                        len,
                    });
                }

                // Not a LaTeX command, treat as regular text
                self.advance();
                Some(Token {
                    kind: SyntaxKind::TEXT,
                    len: 1,
                })
            }

            // Inline code span: consume until matching number of backticks
            '`' => {
                let start_pos = self.pos;
                let tick_count = self.advance_while(|c| c == '`');

                while self.pos < self.input.len() {
                    if self.starts_with(&"`".repeat(tick_count)) {
                        self.advance_while(|c| c == '`');
                        break;
                    } else {
                        self.advance();
                    }
                }
                let len = self.pos - start_pos;

                Some(Token {
                    kind: SyntaxKind::CodeSpan,
                    len,
                })
            }

            '$' => {
                // Detect inline ($...$) and block ($$...$$) math
                let start_pos = self.pos;
                let dollar_count = self.advance_while(|c| c == '$');
                if dollar_count >= 2 {
                    return Some(Token {
                        kind: SyntaxKind::BlockMathMarker,
                        len: dollar_count,
                    });
                }

                // Single '$': apply Pandoc-like heuristics.
                // - If escaped (\$), treat as TEXT.
                // - If immediately followed by a digit, treat as TEXT and
                //   consume the numeric run (e.g., $20,000, $30.).
                // - Otherwise, treat as InlineMathMarker.
                let prev_char = if start_pos == 0 {
                    None
                } else {
                    self.input[..start_pos].chars().next_back()
                };

                if prev_char == Some('\\') {
                    return Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    });
                }

                let next_char = self.current_char();
                if matches!(next_char, Some(c) if c.is_ascii_digit()) {
                    // Consume number-like sequence following the dollar
                    self.advance_while(|c| c.is_ascii_digit() || c == ',' || c == '.');
                    let len = self.pos - start_pos;
                    return Some(Token {
                        kind: SyntaxKind::TEXT,
                        len,
                    });
                }

                Some(Token {
                    kind: SyntaxKind::InlineMathMarker,
                    len: 1,
                })
            }

            '^' if self.starts_with("^[") => {
                self.advance(); // consume ^
                self.advance(); // consume [
                Some(Token {
                    kind: SyntaxKind::InlineFootnoteStart,
                    len: 2,
                })
            }

            '-' | '+' if (self.starts_with("---") || self.starts_with("+++")) => {
                let start_pos = self.pos;
                let ch = self.current_char().unwrap();
                let len = self.advance_while(|c| c == ch);
                let is_start_of_file = start_pos == 0;
                let is_after_newline = start_pos > 0 && self.input[..start_pos].ends_with('\n');
                if (is_start_of_file || is_after_newline) && len == 3 {
                    Some(Token {
                        kind: SyntaxKind::FrontmatterDelim,
                        len,
                    })
                } else {
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len,
                    })
                }
            }

            '<' if self.starts_with("<!--") => {
                self.advance(); // consume <
                self.advance(); // consume !
                self.advance(); // consume -
                self.advance(); // consume -
                Some(Token {
                    kind: SyntaxKind::CommentStart,
                    len: 4,
                })
            }

            '-' if self.starts_with("-->") => {
                self.advance(); // consume -
                self.advance(); // consume -
                self.advance(); // consume >
                Some(Token {
                    kind: SyntaxKind::CommentEnd,
                    len: 3,
                })
            }

            '!' if self.starts_with("![") => {
                let start_pos = self.pos;
                self.advance(); // consume !
                self.advance(); // consume [
                // Find closing ]
                while let Some(ch) = self.current_char() {
                    self.advance();
                    if ch == ']' {
                        break;
                    }
                }
                // If next char is (, try to consume up to )
                if self.current_char() == Some('(') {
                    self.advance(); // consume (
                    while let Some(ch) = self.current_char() {
                        self.advance();
                        if ch == ')' {
                            break;
                        }
                    }
                    let len = self.pos - start_pos;
                    return Some(Token {
                        kind: SyntaxKind::ImageLink,
                        len,
                    });
                }
                // Otherwise, treat as ImageLinkStart
                let len = self.pos - start_pos;
                Some(Token {
                    kind: SyntaxKind::ImageLinkStart,
                    len,
                })
            }

            '[' => {
                // Try to lex a full [text](url) as a single Link token
                let start_pos = self.pos;
                self.advance(); // consume [
                // Find closing ]
                while let Some(ch) = self.current_char() {
                    self.advance();
                    if ch == ']' {
                        break;
                    }
                }
                // If next char is (, try to consume up to )
                if self.current_char() == Some('(') {
                    self.advance(); // consume (
                    while let Some(ch) = self.current_char() {
                        self.advance();
                        if ch == ')' {
                            break;
                        }
                    }
                    let len = self.pos - start_pos;
                    return Some(Token {
                        kind: SyntaxKind::Link,
                        len,
                    });
                }
                // Otherwise, treat as LinkStart
                let len = self.pos - start_pos;
                Some(Token {
                    kind: SyntaxKind::LinkStart,
                    len,
                })
            }

            ']' => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::InlineFootnoteEnd,
                    len: 1,
                })
            }

            '>' => {
                if self.is_block_quote_marker() {
                    self.advance();
                    Some(Token {
                        kind: SyntaxKind::BlockQuoteMarker,
                        len: 1,
                    })
                } else {
                    // Not a valid block quote marker here; treat as text
                    self.advance();
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    })
                }
            }

            '-' | '+' | '*' if self.is_list_marker() => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::ListMarker,
                    len: 1,
                })
            }

            '{' => {
                // Attribute: {...} after any element
                let start_pos = self.pos;
                self.advance(); // consume {
                while let Some(c) = self.current_char() {
                    self.advance();
                    if c == '}' {
                        break;
                    }
                }
                let len = self.pos - start_pos;

                Some(Token {
                    kind: SyntaxKind::Attribute,
                    len,
                })
            }

            '0'..='9' if self.is_numbered_list_marker() => {
                let mut len = 0;
                // Consume digits
                while let Some(ch) = self.current_char() {
                    if ch.is_ascii_digit() {
                        self.advance();
                        len += 1;
                    } else {
                        break;
                    }
                }
                // Consume the dot
                if self.current_char() == Some('.') {
                    self.advance();
                    len += 1;
                }
                Some(Token {
                    kind: SyntaxKind::ListMarker,
                    len,
                })
            }

            _ => {
                // Regular text - advance until we hit something special
                let start = self.pos;
                while let Some(c) = self.current_char() {
                    // Base stop set
                    if matches!(
                        c,
                        '\n' | ' ' | '^' | '\t' | '\r' | '`' | '~' | '$' | '[' | ']' | '\\'
                    ) {
                        break;
                    }
                    // Stop before HTML comment delimiters so they can be tokenized
                    if c == '<' && self.starts_with("<!--") {
                        break;
                    }
                    if c == '-' && self.starts_with("-->") {
                        break;
                    }
                    self.advance();
                }
                let len = self.pos - start;

                // If we didn't advance, advance by one character to prevent infinite loop
                if len == 0 {
                    self.advance();
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    })
                } else {
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len,
                    })
                }
            }
        }
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    let mut iterations = 0;

    while let Some(token) = lexer.next_token() {
        iterations += 1;
        debug_assert!(
            iterations <= 10000,
            "Too many iterations in tokenizer! Input: {:?}, pos: {}",
            input,
            lexer.pos
        );
        log::trace!(
            "Token {}: {:?} (len: {})",
            iterations,
            token.kind,
            token.len
        );
        tokens.push(token);
    }

    log::debug!("Tokenization complete. {} tokens generated.", tokens.len());
    tokens
}

#[cfg(test)]
mod tests;
