use crate::syntax::SyntaxKind;

#[test]
fn lexer_math_block_tokens() {
    let input = "$$\nf(x)=x^2\n$$ {#eq:foobar}\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::BlockMathMarker, // $$
            SyntaxKind::NEWLINE,         //
            SyntaxKind::TEXT,            // f(x) = x^2
            SyntaxKind::NEWLINE,         //
            SyntaxKind::BlockMathMarker, // $$
            SyntaxKind::WHITESPACE,      //
            SyntaxKind::Label,           // {#eq:foobar}
            SyntaxKind::NEWLINE,         //
        ]
    );
}

#[test]
fn lexer_inline_math_tokens() {
    let input = "This is $x^2$ inline math.";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::TEXT,             // This
            SyntaxKind::WHITESPACE,       //
            SyntaxKind::TEXT,             // is
            SyntaxKind::WHITESPACE,       //
            SyntaxKind::InlineMathMarker, // $
            SyntaxKind::TEXT,             // x^2
            SyntaxKind::InlineMathMarker, // $
            SyntaxKind::WHITESPACE,       //
            SyntaxKind::TEXT,             // inline
            SyntaxKind::WHITESPACE,       //
            SyntaxKind::TEXT,             // math.
        ]
    );
}

#[test]
fn lexer_comment_end_bug() {
    let input = "-->";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    // Expect a single CommentEnd token
    assert_eq!(
        kinds,
        vec![SyntaxKind::CommentEnd],
        "Lexer should produce CommentEnd for '-->'"
    );
}

#[test]
fn lexer_triple_dollar_block_math() {
    let input = "$$$\nf(x)=x^2\n$$$\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            SyntaxKind::BlockMathMarker, // $$$
            SyntaxKind::NEWLINE,         //
            SyntaxKind::TEXT,            // f(x)=x^2
            SyntaxKind::NEWLINE,         //
            SyntaxKind::BlockMathMarker, // $$$
            SyntaxKind::NEWLINE,         //
        ],
        "Lexer should treat $$$ as BlockMathMarker"
    );
}

#[test]
fn lexer_escaped_dollar_is_text() {
    let input = r#"foo \$ bar $baz$"#;
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert!(
        kinds.contains(&SyntaxKind::TEXT),
        "Escaped dollar should be TEXT"
    );
    assert!(
        kinds.contains(&SyntaxKind::InlineMathMarker),
        "Unescaped $ should be InlineMathMarker"
    );
}
