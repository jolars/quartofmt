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

            '-' if self.starts_with("---") => {
                let len = self.advance_while(|c| c == '-');
                // Only treat as frontmatter delimiter if it's exactly 3 or more dashes
                if len >= 3 {
                    Some(Token {
                        kind: SyntaxKind::FrontmatterDelim,
                        len,
                    })
                } else {
                    // This shouldn't happen since we check starts_with("---")
                    // but let's handle it safely
                    Some(Token {
                        kind: SyntaxKind::TEXT,
                        len: 1,
                    })
                }
            }

            '+' if self.starts_with("+++") => {
                let len = self.advance_while(|c| c == '+');
                Some(Token {
                    kind: SyntaxKind::FrontmatterDelim,
                    len,
                })
            }

            '>' => {
                self.advance();
                Some(Token {
                    kind: SyntaxKind::BlockQuoteMarker,
                    len: 1,
                })
            }

            _ => {
                // Regular text - advance until we hit something special
                let len = self.advance_while(|c| {
                    !matches!(
                        c,
                        '\n' | ' ' | '\t' | '\r' | '`' | '~' | ':' | '$' | '-' | '+'
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
        log::debug!(
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
