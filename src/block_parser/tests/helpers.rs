use crate::block_parser::BlockParser;
use crate::syntax::{SyntaxKind, SyntaxNode};

pub fn parse_blocks(input: &str) -> SyntaxNode {
    BlockParser::new(input).parse()
}

pub fn find_first(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    node.descendants().find(|n| n.kind() == kind)
}

pub fn get_blocks(node: &SyntaxNode) -> Vec<SyntaxNode> {
    let document = node
        .children()
        .find(|n| n.kind() == SyntaxKind::DOCUMENT)
        .unwrap();
    let blocks: Vec<SyntaxNode> = document.children().collect();
    blocks
}

pub fn assert_block_kinds(input: &str, expected: &[SyntaxKind]) {
    let node = parse_blocks(input);
    let blocks = get_blocks(&node);
    let actual: Vec<_> = blocks.iter().map(|n| n.kind()).collect();
    assert_eq!(
        actual, expected,
        "Block kinds did not match for input:\n{}",
        input
    );
}
