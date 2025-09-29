use crate::block_parser::BlockParser;
use crate::syntax::{SyntaxKind, SyntaxNode};

fn parse_blocks(input: &str) -> SyntaxNode {
    BlockParser::new(input).parse()
}

fn find_first(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    node.descendants().find(|n| n.kind() == kind)
}

fn get_heading_content(node: &SyntaxNode) -> Option<String> {
    find_first(node, SyntaxKind::HeadingContent).map(|n| n.text().to_string())
}

fn get_blocks(node: &SyntaxNode) -> Vec<SyntaxNode> {
    let document = node
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .unwrap();
    let blocks: Vec<SyntaxNode> = document.children().collect();
    blocks
}

#[test]
fn parses_simple_atx_heading() {
    let node = parse_blocks("# Heading\n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "Heading");
}

#[test]
fn empty_atx_heading() {
    let node = parse_blocks("# \n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "");
}

#[test]
fn parses_atx_heading_with_leading_spaces() {
    let node = parse_blocks("  # Leading spaces\n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "Leading spaces");
}

#[test]
fn parses_atx_heading_with_multiple_hashes() {
    let node = parse_blocks("### Subheading\n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "Subheading");
}

#[test]
fn parses_atx_heading_with_trailing_hashes() {
    let node = parse_blocks("### Foo Bar ###\n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "Foo Bar");
}

#[test]
fn does_not_parse_with_four_leading_spaces() {
    let node = parse_blocks("    # Not a heading\n");
    assert!(find_first(&node, SyntaxKind::Heading).is_none());
}

#[test]
fn requires_blank_line_before_heading() {
    let node = parse_blocks("text\n# Heading\n");
    assert!(find_first(&node, SyntaxKind::Heading).is_none());
}

#[test]
fn parses_heading_at_start_of_document() {
    let node = parse_blocks("# Start\n");
    let content = get_heading_content(&node).unwrap();
    assert_eq!(content, "Start");
}

#[test]
fn parses_multiple_headings() {
    let node = parse_blocks("# First\n\n## Second\n");
    let mut headings = node
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::HeadingContent);
    assert_eq!(headings.next().unwrap().text(), "First");
    assert_eq!(headings.next().unwrap().text(), "Second");
}

#[test]
fn blankline_between_paragraphs() {
    let node = parse_blocks("Paragraph 1\n\nParagraph 2\n");
    let blocks = get_blocks(&node);

    println!("{:#?}", blocks);

    assert_eq!(blocks.len(), 3); // Paragraph, BlankLine, Paragraph
}
