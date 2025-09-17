use crate::lexer::tokenize;
use crate::syntax::SyntaxKind;

fn token_texts(input: &str) -> Vec<(SyntaxKind, String)> {
    let tokens = tokenize(input);
    let mut out = Vec::with_capacity(tokens.len());
    let mut off = 0usize;
    for t in tokens {
        let s = &input[off..off + t.len];
        out.push((t.kind, s.to_string()));
        off += t.len;
    }
    out
}

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

#[test]
fn lexer_atx_heading_tokens_basic() {
    let input = "## A level-two heading\n";
    let toks = token_texts(input);

    assert!(!toks.is_empty());
    // First token should be TEXT of only hashes
    assert_eq!(toks[0].0, SyntaxKind::TEXT);
    assert_eq!(toks[0].1, "##");

    // Second token is a space
    assert_eq!(toks[1].0, SyntaxKind::WHITESPACE);
    assert_eq!(toks[1].1, " ");

    // Last token should be NEWLINE
    assert_eq!(toks.last().unwrap().0, SyntaxKind::NEWLINE);
}

#[test]
fn lexer_atx_heading_with_trailing_hashes() {
    let input = "### A level-three heading ###\n";
    let toks = token_texts(input);

    // First token should be TEXT of only hashes
    assert_eq!(toks[0].0, SyntaxKind::TEXT);
    assert_eq!(toks[0].1, "###");

    // Ensure we got a NEWLINE at end
    assert_eq!(toks.last().unwrap().0, SyntaxKind::NEWLINE);
}

#[test]
fn lexer_setext_heading_dashes_not_frontmatter() {
    let input = "A level-two heading\n-------------------\n";
    let toks = token_texts(input);

    // Ensure no FrontmatterDelim is produced for 5+ dashes
    assert!(
        toks.iter().all(|(k, _)| *k != SyntaxKind::FrontmatterDelim),
        "setext underline should not be treated as frontmatter delimiter"
    );

    // Find the token after the first NEWLINE: it should be TEXT of only '-'
    let mut i = 0usize;
    while i < toks.len() && toks[i].0 != SyntaxKind::NEWLINE {
        i += 1;
    }
    assert!(i + 1 < toks.len(), "expected tokens after first newline");
    let (k, s) = (&toks[i + 1].0, toks[i + 1].1.trim().to_string());
    assert_eq!(*k, SyntaxKind::TEXT);
    assert!(!s.is_empty() && s.chars().all(|c| c == '-'));
}

#[test]
fn lexer_setext_heading_equals() {
    let input = "A level-one heading\n====================\n";
    let toks = token_texts(input);

    // Ensure token after first newline is TEXT of only '='
    let mut i = 0usize;
    while i < toks.len() && toks[i].0 != SyntaxKind::NEWLINE {
        i += 1;
    }
    assert!(i + 1 < toks.len(), "expected tokens after first newline");
    let (k, s) = (&toks[i + 1].0, toks[i + 1].1.trim().to_string());
    assert_eq!(*k, SyntaxKind::TEXT);
    assert!(!s.is_empty() && s.chars().all(|c| c == '='));
}
