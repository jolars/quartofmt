use crate::parser::parse;
use crate::syntax::SyntaxKind;

#[test]
fn math_block_structure() {
    let input = "$$\nf(x) = x^2\n$$ {#eq:foobar}\n";
    let tree = parse(input);
    let root = tree;
    let document = root
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");
    let math_block = document
        .children()
        .find(|n| n.kind() == SyntaxKind::MathBlock)
        .expect("MathBlock node");
    let children: Vec<_> = math_block.children().map(|n| n.kind()).collect();
    assert_eq!(
        children,
        vec![
            SyntaxKind::BlockMathMarker,
            SyntaxKind::MathContent,
            SyntaxKind::BlockMathMarker,
            SyntaxKind::Attribute,
        ]
    );
}

#[test]
fn link_should_be_inside_paragraph() {
    let input = "[A network graph. Different edges \"fail\" independently with probability $p$.](../images/networkfig.png){width=70%}\nA word\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");
    let mut paragraphs = document
        .children()
        .filter(|n| n.kind() == SyntaxKind::PARAGRAPH);

    let first_paragraph = paragraphs.next().expect("First paragraph");
    let paragraph_text = first_paragraph.text().to_string();

    assert!(
        paragraph_text.contains("[A network graph."),
        "Link should be inside paragraph node"
    );
    assert!(
        paragraph_text.contains("A word"),
        "Second line should be inside paragraph node"
    );
}

#[test]
fn link_with_attribute_should_include_attribute_in_link_node() {
    let input = "[foo](bar){.class}\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");
    let paragraph = document
        .children()
        .find(|n| n.kind() == SyntaxKind::PARAGRAPH)
        .expect("PARAGRAPH node");
    let link = paragraph
        .children()
        .find(|n| n.kind() == SyntaxKind::Link)
        .expect("Link node");

    let attr = link.children().find(|n| n.kind() == SyntaxKind::Attribute);

    assert!(
        attr.is_some(),
        "Attribute should be included as a child of the Link node"
    );
}

#[test]
fn html_comment_then_text_parses_correctly() {
    let input = "<!--foo-->bar\n";
    let tree = parse(input);

    let document = tree
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");

    let kinds: Vec<_> = document.children().map(|n| n.kind()).collect();
    assert_eq!(kinds.first(), Some(&SyntaxKind::Comment));
    assert_eq!(kinds.get(1), Some(&SyntaxKind::PARAGRAPH));

    let para = document
        .children()
        .find(|n| n.kind() == SyntaxKind::PARAGRAPH)
        .expect("PARAGRAPH node");
    assert_eq!(para.text().to_string(), "bar\n");
}

#[test]
fn lazy_block_quote_paragraph() {
    let input = "> This is a block quote. This paragraph has two lines. It has a bit of text, and then some other\ntext. It is a lazy block quote. It should be formatted as a single paragraph\nwith leading > characters, indented by one space.\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");
    let block_quote = document
        .children()
        .find(|n| n.kind() == SyntaxKind::BlockQuote)
        .expect("BlockQuote node");
    let paragraphs: Vec<_> = block_quote
        .children()
        .filter(|n| n.kind() == SyntaxKind::PARAGRAPH)
        .collect();
    assert_eq!(
        paragraphs.len(),
        1,
        "Lazy block quote should be parsed as a single paragraph"
    );
    let para_text = paragraphs[0].text().to_string();
    assert!(
        para_text.contains("lazy block quote"),
        "Paragraph text should include lazy lines"
    );
}

#[test]
fn double_blockquote_not_nested_without_blank_line() {
    let input = "> This is a block quote.\n>> Not nested, since `blank_before_blockquote` is enabled by default\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == crate::syntax::SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");

    // Should be a single BlockQuote node
    let block_quotes: Vec<_> = document
        .children()
        .filter(|n| n.kind() == crate::syntax::SyntaxKind::BlockQuote)
        .collect();
    assert_eq!(
        block_quotes.len(),
        1,
        "Should only produce a single block quote (not nested) without blank line before"
    );

    // The block quote should contain both lines as a single paragraph
    let para_text = block_quotes[0].text().to_string();
    assert!(
        para_text.contains("Not nested"),
        "Block quote should include the second line"
    );
}

#[test]
fn nested_blockquote_with_blank_line_is_nested() {
    let input = "> This is a block quote.\n>\n> > A block quote within a block quote, which spans many lines. This is the second sentence, and it should lead to wrapping.\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == crate::syntax::SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");

    // Exactly one top-level BlockQuote
    let top_level_bqs: Vec<_> = document
        .children()
        .filter(|n| n.kind() == crate::syntax::SyntaxKind::BlockQuote)
        .collect();
    assert_eq!(
        top_level_bqs.len(),
        1,
        "There should be a single outer BlockQuote"
    );

    let outer = &top_level_bqs[0];
    // It should contain a BlankLine and then a nested BlockQuote
    let mut has_blank = false;
    let mut nested: Option<_> = None;
    for c in outer.children() {
        if c.kind() == crate::syntax::SyntaxKind::BlankLine {
            has_blank = true;
        }
        if c.kind() == crate::syntax::SyntaxKind::BlockQuote {
            nested = Some(c);
        }
    }
    assert!(
        has_blank,
        "Outer BlockQuote should contain a quoted blank line"
    );
    let nested = nested.expect("Nested BlockQuote node should exist");

    // Nested block quote should have a paragraph with the expected text
    let para = nested
        .children()
        .find(|n| n.kind() == crate::syntax::SyntaxKind::PARAGRAPH)
        .expect("Nested paragraph");
    let txt = para.text().to_string();
    assert!(
        txt.contains("A block quote within a block quote") && txt.contains("second sentence"),
        "Nested paragraph text should be present"
    );
}

#[test]
fn headerless_table_without_closing_dashes_is_not_table() {
    // Headerless simple tables must end with a dashed line; otherwise it is not a table.
    let input = "-------     ------ ----------   -------\n\
                 Text not part of table\n";
    let tree = parse(input);
    let document = tree
        .children()
        .find(|n| n.kind() == crate::syntax::SyntaxKind::DOCUMENT)
        .expect("DOCUMENT node");
    let has_simple_table = document
        .children()
        .any(|n| n.kind() == crate::syntax::SyntaxKind::SimpleTable);
    assert!(
        !has_simple_table,
        "Headerless simple table must have a closing dashed line"
    );
}
