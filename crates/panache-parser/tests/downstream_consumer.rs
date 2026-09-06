use panache_parser::parser::SyntaxErrorSource;
use panache_parser::syntax::{
    AstNode, BlockNode, CalloutKind, CellOptionResolution, CodeBlock, InlineNode, Link, SyntaxKind,
    Table, TableAlignment, YamlNode,
};
use panache_parser::{Flavor, ParserOptions, parse_document};

#[test]
fn typed_traversal_preserves_semantic_children_and_unknown_nodes() {
    let source = "# Heading\n\nParagraph with *emphasis*.\n\n<div>raw</div>\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Gfm)));

    assert!(parsed.errors().is_empty());
    let blocks = parsed.document().block_nodes().collect::<Vec<_>>();
    assert!(matches!(blocks[0], BlockNode::Heading(_)));
    assert!(matches!(blocks[1], BlockNode::Trivia(_)));

    let paragraph = blocks
        .iter()
        .find_map(|block| match block {
            BlockNode::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .expect("paragraph");
    assert!(
        paragraph
            .inline_nodes()
            .any(|inline| matches!(inline, InlineNode::Emphasis(_)))
    );

    let unknown = blocks
        .iter()
        .find_map(|block| match block {
            BlockNode::Unknown(unknown) => Some(unknown),
            _ => None,
        })
        .expect("raw HTML remains visible");
    assert_eq!(unknown.source_text(), "<div>raw</div>\n");
    assert_eq!(unknown.range_text(source), "<div>raw</div>\n");
}

#[test]
fn inline_link_destination_exposes_url_and_title_ranges() {
    let source = "[label](<a b> \"A title\")\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Gfm)));
    let link = parsed
        .document()
        .syntax()
        .descendants()
        .find_map(Link::cast)
        .expect("link");
    let destination = link.dest().expect("destination");

    assert_eq!(destination.url_content(), "a b");
    assert_eq!(destination.title().as_deref(), Some("A title"));
    assert_eq!(destination.range_text(source), "<a b> \"A title\"");
    assert_eq!(destination.url_range_text(source), "a b");
    assert_eq!(
        destination.title_range_text(source).as_deref(),
        Some("A title")
    );
    assert_eq!(
        destination
            .syntax()
            .children()
            .map(|child| child.kind())
            .collect::<Vec<_>>(),
        vec![SyntaxKind::LINK_DEST_URL, SyntaxKind::LINK_DEST_TITLE]
    );
}

#[test]
fn executable_cell_retains_yaml_values_provenance_and_code_ranges() {
    let source = "---\ntitle: Example\n---\n\n```{python #setup .hidden, echo=true, fig.cap=\"inline\"}\n#| echo: false\n#| fig-subcap:\n#|   - A\n#|   - B\nprint('ok')\n```\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Quarto)));
    let frontmatter = parsed.document().frontmatter().expect("frontmatter");
    assert_eq!(
        frontmatter
            .document()
            .and_then(|document| document.block_map())
            .and_then(|map| map.value_of("title"))
            .and_then(|value| value.as_scalar())
            .map(|scalar| scalar.value())
            .as_deref(),
        Some("Example")
    );

    let code_block = parsed
        .document()
        .syntax()
        .descendants()
        .find_map(CodeBlock::cast)
        .expect("code block");
    let cell = code_block.executable_cell().expect("executable cell");
    assert_eq!(cell.language().as_deref(), Some("python"));
    let (identifier, identifier_range) = cell.identifier().expect("identifier");
    assert_eq!(identifier, "setup");
    let identifier_start: usize = identifier_range.start().into();
    let identifier_end: usize = identifier_range.end().into();
    assert_eq!(&source[identifier_start..identifier_end], "setup");
    let classes = cell.classes();
    assert_eq!(classes[0].0, "hidden");
    let class_start: usize = classes[0].1.start().into();
    let class_end: usize = classes[0].1.end().into();
    assert_eq!(&source[class_start..class_end], "hidden");
    assert_eq!(cell.code_source(), "print('ok')\n");
    let code_range = cell.code_range().expect("code range");
    let start: usize = code_range.start().into();
    let end: usize = code_range.end().into();
    assert_eq!(&source[start..end], "print('ok')\n");

    let declarations = cell.option_declarations();
    assert_eq!(declarations.len(), 4);
    let subcaptions = declarations
        .iter()
        .find(|option| option.key() == Some("fig-subcap"))
        .expect("fig-subcap");
    assert!(matches!(
        subcaptions.yaml_value(),
        Some(YamlNode::BlockSequence(_))
    ));

    let echo = cell
        .resolved_options()
        .into_iter()
        .find(|option| option.key() == "echo")
        .expect("resolved echo");
    let CellOptionResolution::Resolved(echo) = echo.resolution() else {
        panic!("inline echo should resolve unambiguously");
    };
    assert_eq!(echo.cooked_value(), Some("true"));
}

#[test]
fn duplicate_winning_cell_options_remain_ambiguous() {
    let source = "```{python, echo=true, echo=false}\npass\n```\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Quarto)));
    let code_block = parsed
        .document()
        .syntax()
        .descendants()
        .find_map(CodeBlock::cast)
        .expect("code block");
    let echo = code_block
        .executable_cell()
        .expect("cell")
        .resolved_options()
        .into_iter()
        .find(|option| option.key() == "echo")
        .expect("echo option");
    assert!(matches!(
        echo.resolution(),
        CellOptionResolution::Ambiguous(options) if options.len() == 2
    ));
}

#[test]
fn tables_and_callouts_have_policy_free_typed_views() {
    let table_source = "| L | C | R | D |\n|:--|:-:|--:|---|\n| a | b | c | d |\n";
    let table = parse_document(table_source, Some(ParserOptions::for_flavor(Flavor::Gfm)))
        .document()
        .block_nodes()
        .find_map(|block| match block {
            BlockNode::Table(Table::Pipe(table)) => Some(table),
            _ => None,
        })
        .expect("pipe table");
    assert_eq!(
        table.alignments(),
        vec![
            TableAlignment::Left,
            TableAlignment::Center,
            TableAlignment::Right,
            TableAlignment::Default,
        ]
    );
    let rows = table.all_rows().collect::<Vec<_>>();
    assert!(rows[0].is_header());
    assert_eq!(rows[0].cells().count(), 4);

    let callout_source = "::: {.callout-warning #careful}\nBody with *markup*.\n:::\n";
    let callout = parse_document(
        callout_source,
        Some(ParserOptions::for_flavor(Flavor::Quarto)),
    )
    .document()
    .block_nodes()
    .find_map(|block| match block {
        BlockNode::FencedDiv(div) => div.quarto_callout(),
        _ => None,
    })
    .expect("Quarto callout");
    assert_eq!(callout.kind(), CalloutKind::Warning);
    assert_eq!(
        callout.attributes().and_then(|attributes| attributes.id()),
        Some("careful".to_string())
    );
    assert!(
        callout
            .block_nodes()
            .any(|block| matches!(block, BlockNode::Paragraph(_)))
    );
}

#[test]
fn nested_cell_source_excludes_container_prefixes() {
    let source = "1.  Item\n\n    ```{python}\n    #| echo: true\n    print('nested')\n    ```\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Quarto)));
    let cell = parsed
        .document()
        .syntax()
        .descendants()
        .find_map(CodeBlock::cast)
        .and_then(|block| block.executable_cell())
        .expect("nested cell");
    assert_eq!(cell.code_source(), "print('nested')\n");
    for segment in cell.code_source_segments() {
        let range = segment.text_range();
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        assert_eq!(&source[start..end], segment.text());
    }
}

#[test]
fn gfm_can_retain_code_only_unresolved_references() {
    let source = "See [`package::item`].\n";
    let mut options = ParserOptions::for_flavor(Flavor::Gfm);
    options.preserve_unresolved_references = true;

    let parsed = parse_document(source, Some(options));
    let paragraph = parsed
        .document()
        .block_nodes()
        .find_map(|block| match block {
            BlockNode::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .expect("paragraph");
    let reference = paragraph
        .inline_nodes()
        .find_map(|inline| match inline {
            InlineNode::UnresolvedReference(reference) => Some(reference),
            _ => None,
        })
        .expect("structured unresolved reference");

    let children = reference.inline_nodes().collect::<Vec<_>>();
    assert_eq!(children.len(), 1);
    let InlineNode::Code(code) = &children[0] else {
        panic!("expected code-only reference");
    };
    assert_eq!(code.content(), "package::item");
}

#[test]
fn malformed_embedded_yaml_errors_keep_host_ranges() {
    let source = "---\ntitle: [broken\n---\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Quarto)));
    let error = parsed.errors().first().expect("YAML error");

    assert_eq!(error.source, SyntaxErrorSource::Yaml);
    let start: usize = error.range.start().into();
    let end: usize = error.range.end().into();
    assert!(start <= end);
    assert!(end <= source.len());
    assert!(source.is_char_boundary(start));
    assert!(source.is_char_boundary(end));
}

#[test]
fn document_reference_definitions_include_nested_containers() {
    let source = "> [site]: https://example.com\n\n[site]\n";
    let parsed = parse_document(source, Some(ParserOptions::for_flavor(Flavor::Gfm)));
    let definitions = parsed
        .document()
        .reference_definitions()
        .collect::<Vec<_>>();

    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].label(), "site");
    assert_eq!(definitions[0].url().as_deref(), Some("https://example.com"));
}
