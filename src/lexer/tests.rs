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

#[test]
fn lexer_code_span_exact_token_sequence() {
    let input = "foo `bar $baz$` qux";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        crate::syntax::SyntaxKind::TEXT, // foo
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::CodeSpan, // `bar $baz$`
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // qux
    ];
    assert_eq!(
        kinds, expected,
        "Lexer should tokenize inline code as CodeSpan"
    );
}

#[test]
fn lexer_multiline_code_span_tokenizes_as_single_code_span() {
    let input = "foo `bar\nbaz $qux$` quux";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        crate::syntax::SyntaxKind::TEXT, // foo
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::CodeSpan, // `bar\nbaz $qux$`
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // quux
    ];
    assert_eq!(
        kinds, expected,
        "Lexer should tokenize multiline inline code as a single CodeSpan"
    );
}

#[test]
fn lexer_list_marker_bol_only() {
    let input = "foo - bar\n- item\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        crate::syntax::SyntaxKind::TEXT,       // foo
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // -
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // bar
        crate::syntax::SyntaxKind::NEWLINE,    //
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // item
        crate::syntax::SyntaxKind::NEWLINE,    //
    ];
    assert_eq!(kinds, expected, "Only BOL '-' should be ListMarker");
}

#[test]
fn lexer_nested_list_tokens() {
    let input = "- Top level\n  - Nested level 1\n    - Nested level 2\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // Top
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // level
        crate::syntax::SyntaxKind::NEWLINE,
        crate::syntax::SyntaxKind::WHITESPACE, // (indent)
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // Nested
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // level
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // 1
        crate::syntax::SyntaxKind::NEWLINE,
        crate::syntax::SyntaxKind::WHITESPACE, // (indent)
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // Nested
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // level
        crate::syntax::SyntaxKind::WHITESPACE,
        crate::syntax::SyntaxKind::TEXT, // 2
        crate::syntax::SyntaxKind::NEWLINE,
    ];
    assert_eq!(
        kinds, expected,
        "Lexer should tokenize nested list markers and indentation correctly"
    );
}
