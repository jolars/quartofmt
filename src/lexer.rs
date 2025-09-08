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

    pub fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn is_list_marker(&self) -> bool {
        // Check if this is a list marker (-, +, *) followed by space
        if let Some(ch) = self.current_char()
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

    pub fn advance_while<F>(&mut self, mut predicate: F) -> usize
    where
        F: FnMut(char) -> bool,
    {
        let start = self.pos;
        let mut iterations = 0;
        while let Some(ch) = self.current_char() {
            iterations += 1;
            if iterations > 1000 {
                panic!(
                    "Infinite loop in advance_while! pos: {}, char: {:?}",
                    self.pos, ch
                );
            }
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

            '`' if self.starts_with("```") => {
                let len = self.advance_while(|c| c == '`');
                Some(Token {
                    kind: SyntaxKind::FenceMarker,
                    len,
                })
            }

            '~' if self.starts_with("~~~") => {
                let len = self.advance_while(|c| c == '~');
                Some(Token {
                    kind: SyntaxKind::FenceMarker,
                    len,
                })
            }

            ':' if self.starts_with(":::") => {
                let len = self.advance_while(|c| c == ':');
                Some(Token {
                    kind: SyntaxKind::DivMarker,
                    len,
                })
            }

            '$' if self.starts_with("$$") => {
                let len = self.advance_while(|c| c == '$');
                Some(Token {
                    kind: SyntaxKind::MathMarker,
                    len,
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

            '>' if self.starts_with("-->") => {
                self.advance(); // consume -
                self.advance(); // consume -
                self.advance(); // consume >
                Some(Token {
                    kind: SyntaxKind::CommentEnd,
                    len: 3,
                })
            }

            '!' if self.starts_with("![") => {
                self.advance(); // consume !
                self.advance(); // consume [
                Some(Token {
                    kind: SyntaxKind::ImageLinkStart,
                    len: 2,
                })
            }

            '[' => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::LinkStart,
                    len: 1,
                })
            }

            '>' => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::BlockQuoteMarker,
                    len: 1,
                })
            }

            '-' | '+' | '*' if self.is_list_marker() => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::ListMarker,
                    len: 1,
                })
            }

            '{' if self.starts_with("{#") => {
                // General label: {#...}
                let start_pos = self.pos;
                self.advance(); // consume {
                self.advance(); // consume #
                // Consume until we hit }
                while let Some(c) = self.current_char() {
                    self.advance();
                    if c == '}' {
                        break;
                    }
                }
                let len = self.pos - start_pos;

                Some(Token {
                    kind: SyntaxKind::Label,
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
                // Check for special case: ! followed by [
                if ch == '!' && self.peek_char(1) == Some('[') {
                    // This will be handled by the ![  case above, but if we get here
                    // just advance one character to avoid infinite loop
                    self.advance();
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    })
                } else if ch.is_ascii_digit() && self.is_numbered_list_marker() {
                    // This will be handled by the numbered list case above
                    self.advance();
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    })
                } else {
                    // Regular text - advance until we hit something special
                    let len = self.advance_while(|c| {
                        !matches!(
                            c,
                            '\n' | ' '
                                | '\t'
                                | '\r'
                                | '`'
                                | '~'
                                | ':'
                                | '$'
                                | '-'
                                | '+'
                                | '['
                                | '>'
                                | '!'
                                | '\\'
                        )
                    });

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
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    let mut iterations = 0;

    while let Some(token) = lexer.next_token() {
        iterations += 1;
        if iterations > 10000 {
            panic!(
                "Too many iterations in tokenizer! Input: {:?}, pos: {}",
                input, lexer.pos
            );
        }
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
