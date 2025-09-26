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
            SyntaxKind::TEXT,            // ^
            SyntaxKind::TEXT,            // 2
            SyntaxKind::NEWLINE,         //
            SyntaxKind::BlockMathMarker, // $$
            SyntaxKind::WHITESPACE,      //
            SyntaxKind::Attribute,       // {#eq:foobar}
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
            SyntaxKind::TEXT,             // x
            SyntaxKind::TEXT,             // 2
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
            SyntaxKind::TEXT,            // ^
            SyntaxKind::TEXT,            // 2
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
        SyntaxKind::TEXT,       // foo
        SyntaxKind::WHITESPACE, //
        SyntaxKind::TEXT,       // -
        SyntaxKind::WHITESPACE, //
        SyntaxKind::TEXT,       // bar
        SyntaxKind::NEWLINE,    //
        SyntaxKind::ListMarker, // -
        SyntaxKind::WHITESPACE, //
        SyntaxKind::TEXT,       // item
        SyntaxKind::NEWLINE,    //
    ];
    assert_eq!(kinds, expected, "Only BOL '-' should be ListMarker");
}

#[test]
fn nested_list_tokens() {
    let input = "- Top level\n  - Nested level 1\n    - Nested level 2\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // Top
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // level
        crate::syntax::SyntaxKind::NEWLINE,    //
        crate::syntax::SyntaxKind::WHITESPACE, // (indent)
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // Nested
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // level
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // 1
        crate::syntax::SyntaxKind::NEWLINE,    //
        crate::syntax::SyntaxKind::WHITESPACE, // (indent)
        crate::syntax::SyntaxKind::ListMarker, // -
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // Nested
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // level
        crate::syntax::SyntaxKind::WHITESPACE, //
        crate::syntax::SyntaxKind::TEXT,       // 2
        crate::syntax::SyntaxKind::NEWLINE,    //
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

#[test]
fn lexer_does_not_treat_hash_number_as_heading() {
    let input = "I like several of their flavors of ice cream:\n#22, for example, and #5.\n";
    let tokens = tokenize(input);
    // Find the token for "#22"
    let hash_token = tokens.iter().find(|t| t.kind == SyntaxKind::TEXT).unwrap();
    // It should be TEXT, not a heading marker
    assert_eq!(hash_token.kind, SyntaxKind::TEXT);
}

#[test]
fn latex_math_and_command() {
    // Create a test string with inline math followed by space and text
    let input = "$\\alpha$ is text";

    // Tokenize the input
    let tokens = tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::InlineMathMarker, // $
        SyntaxKind::LatexCommand,     // \alpha
        SyntaxKind::InlineMathMarker, // $
        SyntaxKind::WHITESPACE,       // (space)
        SyntaxKind::TEXT,             // is
        SyntaxKind::WHITESPACE,       // (space)
        SyntaxKind::TEXT,             // text
    ];

    assert_eq!(
        kinds, expected,
        "Lexer should correctly tokenize LaTeX math and commands"
    );
}

#[test]
fn handle_actual_dollar() {
    let input = "Costs $20,000 not $30.";
    let tokens = tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::TEXT,       // Costs
        SyntaxKind::WHITESPACE, // (space)
        SyntaxKind::TEXT,       // $20,000
        SyntaxKind::WHITESPACE, // (space)
        SyntaxKind::TEXT,       // not
        SyntaxKind::WHITESPACE, // (space)
        SyntaxKind::TEXT,       // $30.
    ];

    assert_eq!(kinds, expected, "Lexer should treat dollar amounts as TEXT");
}

#[test]
fn lexer_blockquote_marker_only_at_bol_and_max_three_spaces() {
    let input = "    > Not a block quote (too much indent)\n\n > Valid block quote (one space)\n\n>Valid block quote\n\nfoo > not a block quote\n";
    let tokens = crate::lexer::tokenize(input);

    // Find all BlockQuoteMarker tokens and their positions
    let positions: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| t.kind == crate::syntax::SyntaxKind::BlockQuoteMarker)
        .map(|(i, _)| i)
        .collect();

    // Should only emit BlockQuoteMarker for the two valid lines (2nd and 3rd logical lines)
    assert_eq!(
        positions.len(),
        2,
        "Should only emit BlockQuoteMarker for valid block quotes"
    );
}

#[test]
fn lexer_blockquote_requires_blank_line_unless_bof() {
    let input = "Intro line\n> Not a block quote (no blank line before)\n\n> Valid block quote (after blank)\n";
    let tokens = crate::lexer::tokenize(input);
    let count = tokens
        .iter()
        .filter(|t| t.kind == crate::syntax::SyntaxKind::BlockQuoteMarker)
        .count();
    assert_eq!(
        count, 1,
        "Only the block quote after a blank line should be recognized"
    );

    // BOF case: allowed without preceding blank line
    let input2 = "> Start of document\nNext line\n";
    let tokens2 = crate::lexer::tokenize(input2);
    let count2 = tokens2
        .iter()
        .filter(|t| t.kind == crate::syntax::SyntaxKind::BlockQuoteMarker)
        .count();
    assert_eq!(
        count2, 1,
        "Block quote at beginning of document should be recognized"
    );
}

#[test]
fn lexer_blockquote_more_than_three_spaces_is_not_marker_even_after_blank() {
    let input = "\n    > Too much indent\n";
    let tokens = crate::lexer::tokenize(input);
    let count = tokens
        .iter()
        .filter(|t| t.kind == crate::syntax::SyntaxKind::BlockQuoteMarker)
        .count();
    assert_eq!(
        count, 0,
        "Indent > 3 spaces should not be a block quote marker"
    );
}

#[test]
fn lexer_nested_blockquote_after_blank_line_has_two_markers_on_line() {
    // After a quoted blank line, a nested quote line with '> >' should emit two markers.
    let input = "> Text\n>\n> > Nested\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::BlockQuoteMarker, // >
        SyntaxKind::WHITESPACE,       // (space)
        SyntaxKind::TEXT,             // Text
        SyntaxKind::NEWLINE,          //
        SyntaxKind::BlockQuoteMarker, // >
        SyntaxKind::NEWLINE,          //
        SyntaxKind::BlockQuoteMarker, // >
        SyntaxKind::WHITESPACE,       // (space)
        SyntaxKind::BlockQuoteMarker, // >
        SyntaxKind::WHITESPACE,       // (space)
        SyntaxKind::TEXT,             // Nested
        SyntaxKind::NEWLINE,          //
    ];
    assert_eq!(
        kinds, expected,
        "Lexer should emit two BlockQuoteMarkers for nested quote"
    );
}

#[test]
fn inline_footnote_tokens() {
    let input = "This is a footnote^[with some text] in a sentence.";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::TEXT,                // This
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // is
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // a
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // footnote
        SyntaxKind::InlineFootnoteStart, // ^[
        SyntaxKind::TEXT,                // with
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // some
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // text
        SyntaxKind::InlineFootnoteEnd,   // ]
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // in
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // a
        SyntaxKind::WHITESPACE,          // (space)
        SyntaxKind::TEXT,                // sentence.
    ];

    assert_eq!(
        kinds, expected,
        "Lexer should tokenize inline footnotes correctly"
    );
}

#[test]
fn horizontal_rule() {
    let input = "Above\n\n---\n\nBelow\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::TEXT,           // Some
        SyntaxKind::NEWLINE,        //
        SyntaxKind::NEWLINE,        //
        SyntaxKind::HorizontalRule, // ---
        SyntaxKind::NEWLINE,        //
        SyntaxKind::NEWLINE,        //
        SyntaxKind::TEXT,           // below
        SyntaxKind::NEWLINE,        //
    ];

    assert_eq!(
        kinds, expected,
        "Lexer should tokenize horizontal rules correctly"
    );
}

#[test]
fn horizontal_rule_spaced() {
    let input = "Above\n\n* * * * *\n\nBelow\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();

    let expected = vec![
        SyntaxKind::TEXT,           // Some
        SyntaxKind::NEWLINE,        //
        SyntaxKind::NEWLINE,        //
        SyntaxKind::HorizontalRule, // * * * * *
        SyntaxKind::NEWLINE,        //
        SyntaxKind::NEWLINE,        //
        SyntaxKind::TEXT,           // below
        SyntaxKind::NEWLINE,        //
    ];

    assert_eq!(
        kinds, expected,
        "Lexer should tokenize horizontal rules with spaces correctly"
    );
}

#[test]
fn frontmatter_with_dots() {
    let input = "---\ntitle: Test\n...\n\nContent\n";
    let tokens = crate::lexer::tokenize(input);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    let expected = vec![
        SyntaxKind::FrontmatterDelim, // ---
        SyntaxKind::NEWLINE,          //
        SyntaxKind::TEXT,             // title
        SyntaxKind::WHITESPACE,       //
        SyntaxKind::TEXT,             // Test
        SyntaxKind::NEWLINE,          //
        SyntaxKind::FrontmatterDelim, // ...
        SyntaxKind::NEWLINE,          //
        SyntaxKind::NEWLINE,          //
        SyntaxKind::TEXT,             // Content
        SyntaxKind::NEWLINE,          //
    ];
    assert_eq!(
        kinds, expected,
        "Lexer should recognize '...' as frontmatter delimiter"
    );
}
