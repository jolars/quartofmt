//! Stable typed traversal for downstream document consumers.
//!
//! These views classify CST children without hiding their source text or
//! requiring consumers to depend on the precise Rowan child layout.

use rowan::NodeOrToken;

use super::*;
use crate::parser::utils::attributes::decode_html_attr_entities;

/// One block-level CST child classified for downstream traversal.
#[non_exhaustive]
pub enum BlockNode {
    YamlMetadata(YamlMetadata),
    PandocTitleBlock(PandocTitleBlock),
    MmdTitleBlock(MmdTitleBlock),
    Paragraph(Paragraph),
    Plain(Plain),
    Heading(Heading),
    BlockQuote(BlockQuote),
    Alert(Alert),
    List(List),
    DefinitionList(DefinitionList),
    LineBlock(LineBlock),
    CodeBlock(CodeBlock),
    FencedDiv(FencedDiv),
    Figure(Figure),
    FootnoteDefinition(FootnoteDefinition),
    ReferenceDefinition(ReferenceDefinition),
    DisplayMath(DisplayMath),
    TexBlock(TexBlock),
    MystDirective(MystDirective),
    Table(Table),
    ThematicBreak(ThematicBreak),
    Trivia(TriviaNode),
    Unknown(UnknownNode),
}

impl BlockNode {
    /// Classify one CST element as a block-level consumer node.
    pub fn cast(element: SyntaxElement) -> Self {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::HORIZONTAL_RULE => Self::ThematicBreak(ThematicBreak(token.into())),
                SyntaxKind::BLANK_LINE | SyntaxKind::WHITESPACE | SyntaxKind::NEWLINE => {
                    Self::Trivia(TriviaNode(token.into()))
                }
                _ => Self::Unknown(UnknownNode(token.into())),
            },
            NodeOrToken::Node(node) => match node.kind() {
                SyntaxKind::YAML_METADATA => {
                    Self::YamlMetadata(YamlMetadata::cast(node).expect("checked YAML metadata"))
                }
                SyntaxKind::PANDOC_TITLE_BLOCK => Self::PandocTitleBlock(
                    PandocTitleBlock::cast(node).expect("checked Pandoc title block"),
                ),
                SyntaxKind::MMD_TITLE_BLOCK => {
                    Self::MmdTitleBlock(MmdTitleBlock::cast(node).expect("checked MMD title block"))
                }
                SyntaxKind::PARAGRAPH => {
                    Self::Paragraph(Paragraph::cast(node).expect("checked paragraph"))
                }
                SyntaxKind::PLAIN => Self::Plain(Plain::cast(node).expect("checked plain")),
                SyntaxKind::HEADING => Self::Heading(Heading::cast(node).expect("checked heading")),
                SyntaxKind::BLOCK_QUOTE => {
                    Self::BlockQuote(BlockQuote::cast(node).expect("checked block quote"))
                }
                SyntaxKind::ALERT => Self::Alert(Alert::cast(node).expect("checked alert")),
                SyntaxKind::LIST => Self::List(List::cast(node).expect("checked list")),
                SyntaxKind::DEFINITION_LIST => Self::DefinitionList(
                    DefinitionList::cast(node).expect("checked definition list"),
                ),
                SyntaxKind::LINE_BLOCK => {
                    Self::LineBlock(LineBlock::cast(node).expect("checked line block"))
                }
                SyntaxKind::CODE_BLOCK => {
                    Self::CodeBlock(CodeBlock::cast(node).expect("checked code block"))
                }
                SyntaxKind::FENCED_DIV => {
                    Self::FencedDiv(FencedDiv::cast(node).expect("checked fenced div"))
                }
                SyntaxKind::FIGURE => Self::Figure(Figure::cast(node).expect("checked figure")),
                SyntaxKind::FOOTNOTE_DEFINITION => Self::FootnoteDefinition(
                    FootnoteDefinition::cast(node).expect("checked footnote definition"),
                ),
                SyntaxKind::REFERENCE_DEFINITION => Self::ReferenceDefinition(
                    ReferenceDefinition::cast(node).expect("checked reference definition"),
                ),
                SyntaxKind::DISPLAY_MATH => {
                    Self::DisplayMath(DisplayMath::cast(node).expect("checked display math"))
                }
                SyntaxKind::TEX_BLOCK => {
                    Self::TexBlock(TexBlock::cast(node).expect("checked TeX block"))
                }
                SyntaxKind::MYST_DIRECTIVE => {
                    Self::MystDirective(MystDirective::cast(node).expect("checked MyST directive"))
                }
                SyntaxKind::PIPE_TABLE
                | SyntaxKind::GRID_TABLE
                | SyntaxKind::SIMPLE_TABLE
                | SyntaxKind::MULTILINE_TABLE => {
                    Self::Table(Table::cast(node).expect("checked table"))
                }
                SyntaxKind::HORIZONTAL_RULE => Self::ThematicBreak(ThematicBreak(node.into())),
                SyntaxKind::BLANK_LINE => Self::Trivia(TriviaNode(node.into())),
                _ => Self::Unknown(UnknownNode(node.into())),
            },
        }
    }

    pub fn syntax_kind(&self) -> SyntaxKind {
        match self {
            Self::YamlMetadata(node) => node.syntax().kind(),
            Self::PandocTitleBlock(node) => node.syntax().kind(),
            Self::MmdTitleBlock(node) => node.syntax().kind(),
            Self::Paragraph(node) => node.syntax().kind(),
            Self::Plain(node) => node.syntax().kind(),
            Self::Heading(node) => node.syntax().kind(),
            Self::BlockQuote(node) => node.syntax().kind(),
            Self::Alert(node) => node.syntax().kind(),
            Self::List(node) => node.syntax().kind(),
            Self::DefinitionList(node) => node.syntax().kind(),
            Self::LineBlock(node) => node.syntax().kind(),
            Self::CodeBlock(node) => node.syntax().kind(),
            Self::FencedDiv(node) => node.syntax().kind(),
            Self::Figure(node) => node.syntax().kind(),
            Self::FootnoteDefinition(node) => node.syntax().kind(),
            Self::ReferenceDefinition(node) => node.syntax().kind(),
            Self::DisplayMath(node) => node.syntax().kind(),
            Self::TexBlock(node) => node.syntax().kind(),
            Self::MystDirective(node) => node.syntax().kind(),
            Self::Table(node) => node.syntax().kind(),
            Self::ThematicBreak(node) => node.syntax_kind(),
            Self::Trivia(node) => node.syntax_kind(),
            Self::Unknown(node) => node.syntax_kind(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            Self::YamlMetadata(node) => node.syntax().text_range(),
            Self::PandocTitleBlock(node) => node.syntax().text_range(),
            Self::MmdTitleBlock(node) => node.syntax().text_range(),
            Self::Paragraph(node) => node.syntax().text_range(),
            Self::Plain(node) => node.syntax().text_range(),
            Self::Heading(node) => node.syntax().text_range(),
            Self::BlockQuote(node) => node.syntax().text_range(),
            Self::Alert(node) => node.syntax().text_range(),
            Self::List(node) => node.syntax().text_range(),
            Self::DefinitionList(node) => node.syntax().text_range(),
            Self::LineBlock(node) => node.syntax().text_range(),
            Self::CodeBlock(node) => node.syntax().text_range(),
            Self::FencedDiv(node) => node.syntax().text_range(),
            Self::Figure(node) => node.syntax().text_range(),
            Self::FootnoteDefinition(node) => node.syntax().text_range(),
            Self::ReferenceDefinition(node) => node.syntax().text_range(),
            Self::DisplayMath(node) => node.syntax().text_range(),
            Self::TexBlock(node) => node.syntax().text_range(),
            Self::MystDirective(node) => node.syntax().text_range(),
            Self::Table(node) => node.syntax().text_range(),
            Self::ThematicBreak(node) => node.text_range(),
            Self::Trivia(node) => node.text_range(),
            Self::Unknown(node) => node.text_range(),
        }
    }

    pub fn source_text(&self) -> String {
        match self {
            Self::YamlMetadata(node) => node.syntax().text().to_string(),
            Self::PandocTitleBlock(node) => node.syntax().text().to_string(),
            Self::MmdTitleBlock(node) => node.syntax().text().to_string(),
            Self::Paragraph(node) => node.syntax().text().to_string(),
            Self::Plain(node) => node.syntax().text().to_string(),
            Self::Heading(node) => node.syntax().text().to_string(),
            Self::BlockQuote(node) => node.syntax().text().to_string(),
            Self::Alert(node) => node.syntax().text().to_string(),
            Self::List(node) => node.syntax().text().to_string(),
            Self::DefinitionList(node) => node.syntax().text().to_string(),
            Self::LineBlock(node) => node.syntax().text().to_string(),
            Self::CodeBlock(node) => node.syntax().text().to_string(),
            Self::FencedDiv(node) => node.syntax().text().to_string(),
            Self::Figure(node) => node.syntax().text().to_string(),
            Self::FootnoteDefinition(node) => node.syntax().text().to_string(),
            Self::ReferenceDefinition(node) => node.syntax().text().to_string(),
            Self::DisplayMath(node) => node.syntax().text().to_string(),
            Self::TexBlock(node) => node.syntax().text().to_string(),
            Self::MystDirective(node) => node.syntax().text().to_string(),
            Self::Table(node) => node.syntax().text().to_string(),
            Self::ThematicBreak(node) => node.source_text(),
            Self::Trivia(node) => node.source_text(),
            Self::Unknown(node) => node.source_text(),
        }
    }
}

/// One inline-level CST child classified for downstream traversal.
#[non_exhaustive]
pub enum InlineNode {
    Text(TextSegment),
    Space(TextSegment),
    SoftBreak(TextSegment),
    HardBreak(TextSegment),
    NonbreakingSpace(TextSegment),
    Emphasis(InlineContainer),
    Strong(InlineContainer),
    Strikeout(InlineContainer),
    Mark(InlineContainer),
    Superscript(InlineContainer),
    Subscript(InlineContainer),
    Code(CodeSpan),
    Link(Link),
    Image(ImageLink),
    AutoLink(AutoLink),
    Math(InlineMath),
    UnresolvedReference(UnresolvedReference),
    Unknown(UnknownNode),
}

impl InlineNode {
    pub fn cast(element: SyntaxElement) -> Self {
        match element {
            NodeOrToken::Token(token) => match token.kind() {
                SyntaxKind::TEXT | SyntaxKind::ESCAPED_CHAR => Self::Text(TextSegment(token)),
                SyntaxKind::WHITESPACE => Self::Space(TextSegment(token)),
                SyntaxKind::NEWLINE => Self::SoftBreak(TextSegment(token)),
                SyntaxKind::HARD_LINE_BREAK => Self::HardBreak(TextSegment(token)),
                SyntaxKind::NONBREAKING_SPACE => Self::NonbreakingSpace(TextSegment(token)),
                _ => Self::Unknown(UnknownNode(token.into())),
            },
            NodeOrToken::Node(node) => match node.kind() {
                SyntaxKind::EMPHASIS => Self::Emphasis(InlineContainer(node)),
                SyntaxKind::STRONG => Self::Strong(InlineContainer(node)),
                SyntaxKind::STRIKEOUT => Self::Strikeout(InlineContainer(node)),
                SyntaxKind::MARK => Self::Mark(InlineContainer(node)),
                SyntaxKind::SUPERSCRIPT => Self::Superscript(InlineContainer(node)),
                SyntaxKind::SUBSCRIPT => Self::Subscript(InlineContainer(node)),
                SyntaxKind::INLINE_CODE => {
                    Self::Code(CodeSpan::cast(node).expect("checked inline code"))
                }
                SyntaxKind::LINK => Self::Link(Link::cast(node).expect("checked link")),
                SyntaxKind::IMAGE_LINK => {
                    Self::Image(ImageLink::cast(node).expect("checked image"))
                }
                SyntaxKind::AUTO_LINK => {
                    Self::AutoLink(AutoLink::cast(node).expect("checked autolink"))
                }
                SyntaxKind::INLINE_MATH => {
                    Self::Math(InlineMath::cast(node).expect("checked inline math"))
                }
                SyntaxKind::UNRESOLVED_REFERENCE => Self::UnresolvedReference(
                    UnresolvedReference::cast(node).expect("checked unresolved reference"),
                ),
                _ => Self::Unknown(UnknownNode(node.into())),
            },
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            Self::Text(node)
            | Self::Space(node)
            | Self::SoftBreak(node)
            | Self::HardBreak(node)
            | Self::NonbreakingSpace(node) => node.text_range(),
            Self::Emphasis(node)
            | Self::Strong(node)
            | Self::Strikeout(node)
            | Self::Mark(node)
            | Self::Superscript(node)
            | Self::Subscript(node) => node.syntax().text_range(),
            Self::Code(node) => node.syntax().text_range(),
            Self::Link(node) => node.syntax().text_range(),
            Self::Image(node) => node.syntax().text_range(),
            Self::AutoLink(node) => node.syntax().text_range(),
            Self::Math(node) => node.syntax().text_range(),
            Self::UnresolvedReference(node) => node.syntax().text_range(),
            Self::Unknown(node) => node.text_range(),
        }
    }

    pub fn syntax_kind(&self) -> SyntaxKind {
        match self {
            Self::Text(node)
            | Self::Space(node)
            | Self::SoftBreak(node)
            | Self::HardBreak(node)
            | Self::NonbreakingSpace(node) => node.syntax_kind(),
            Self::Emphasis(node)
            | Self::Strong(node)
            | Self::Strikeout(node)
            | Self::Mark(node)
            | Self::Superscript(node)
            | Self::Subscript(node) => node.syntax().kind(),
            Self::Code(node) => node.syntax().kind(),
            Self::Link(node) => node.syntax().kind(),
            Self::Image(node) => node.syntax().kind(),
            Self::AutoLink(node) => node.syntax().kind(),
            Self::Math(node) => node.syntax().kind(),
            Self::UnresolvedReference(node) => node.syntax().kind(),
            Self::Unknown(node) => node.syntax_kind(),
        }
    }

    pub fn source_text(&self) -> String {
        match self {
            Self::Text(node)
            | Self::Space(node)
            | Self::SoftBreak(node)
            | Self::HardBreak(node)
            | Self::NonbreakingSpace(node) => node.raw().to_string(),
            Self::Emphasis(node)
            | Self::Strong(node)
            | Self::Strikeout(node)
            | Self::Mark(node)
            | Self::Superscript(node)
            | Self::Subscript(node) => node.syntax().text().to_string(),
            Self::Code(node) => node.syntax().text().to_string(),
            Self::Link(node) => node.syntax().text().to_string(),
            Self::Image(node) => node.syntax().text().to_string(),
            Self::AutoLink(node) => node.syntax().text().to_string(),
            Self::Math(node) => node.syntax().text().to_string(),
            Self::UnresolvedReference(node) => node.syntax().text().to_string(),
            Self::Unknown(node) => node.source_text(),
        }
    }
}

/// A source token with a consumer-oriented decoded value.
pub struct TextSegment(SyntaxToken);

impl TextSegment {
    pub fn syntax_kind(&self) -> SyntaxKind {
        self.0.kind()
    }

    pub fn raw(&self) -> &str {
        self.0.text()
    }

    pub fn decoded(&self) -> String {
        match self.0.kind() {
            SyntaxKind::ESCAPED_CHAR => self.0.text().chars().skip(1).collect(),
            SyntaxKind::NONBREAKING_SPACE => "\u{a0}".to_string(),
            SyntaxKind::WHITESPACE => " ".to_string(),
            SyntaxKind::HARD_LINE_BREAK => "\n".to_string(),
            SyntaxKind::TEXT => decode_html_attr_entities(self.0.text()).into_owned(),
            _ => self.0.text().to_string(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }
}

/// A node whose delimiters are structural and whose payload contains inlines.
pub struct InlineContainer(SyntaxNode);

impl InlineContainer {
    pub fn syntax(&self) -> &SyntaxNode {
        &self.0
    }

    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(&self.0)
    }
}

/// A thematic break token or node.
pub struct ThematicBreak(SyntaxElement);

impl ThematicBreak {
    pub fn syntax_kind(&self) -> SyntaxKind {
        self.0.kind()
    }

    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }

    pub fn source_text(&self) -> String {
        element_text(&self.0)
    }
}

/// Whitespace or blank-line trivia retained in typed traversal.
pub struct TriviaNode(SyntaxElement);

impl TriviaNode {
    pub fn syntax_kind(&self) -> SyntaxKind {
        self.0.kind()
    }

    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }

    pub fn source_text(&self) -> String {
        element_text(&self.0)
    }
}

/// A CST element without a dedicated typed consumer variant.
pub struct UnknownNode(SyntaxElement);

impl UnknownNode {
    pub fn syntax_kind(&self) -> SyntaxKind {
        self.0.kind()
    }

    pub fn syntax_name(&self) -> String {
        format!("{:?}", self.syntax_kind())
    }

    pub fn text_range(&self) -> TextRange {
        self.0.text_range()
    }

    pub fn source_text(&self) -> String {
        element_text(&self.0)
    }

    pub fn range_text<'a>(&self, source: &'a str) -> &'a str {
        let range = self.text_range();
        let start: usize = range.start().into();
        let end: usize = range.end().into();
        &source[start..end]
    }

    pub fn syntax_element(&self) -> &SyntaxElement {
        &self.0
    }
}

impl Document {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), |_| false)
    }

    pub fn frontmatter(&self) -> Option<YamlMetadata> {
        self.syntax().children().find_map(YamlMetadata::cast)
    }

    pub fn reference_definitions(&self) -> impl Iterator<Item = ReferenceDefinition> {
        self.syntax()
            .descendants()
            .filter_map(ReferenceDefinition::cast)
    }
}

impl Paragraph {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl Plain {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl Heading {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        self.content()
            .map(|content| content.inline_nodes().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn attributes(&self) -> Option<AttributeNode> {
        self.syntax().children().find_map(AttributeNode::cast)
    }
}

impl HeadingContent {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl BlockQuote {
    /// Return the GitHub-style alert contained by this block quote, if any.
    pub fn alert(&self) -> Option<Alert> {
        self.syntax().children().find_map(Alert::cast)
    }

    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), is_container_marker)
    }
}

impl ListItem {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), is_container_marker)
    }
}

impl Alert {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), is_container_marker)
    }
}

impl FencedDiv {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), is_container_marker)
    }
}

impl QuartoCallout {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        self.fenced_div().block_nodes()
    }
}

impl TableCell {
    pub fn block_nodes(&self) -> std::vec::IntoIter<BlockNode> {
        block_nodes(self.syntax(), is_container_marker)
    }

    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl TableCaption {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl LinkText {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl ImageAlt {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        inline_nodes(self.syntax())
    }
}

impl Link {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        self.text()
            .map(|text| text.inline_nodes().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn attributes(&self) -> Option<AttributeNode> {
        self.syntax().children().find_map(AttributeNode::cast)
    }
}

impl ImageLink {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        self.alt()
            .map(|alt| alt.inline_nodes().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn attributes(&self) -> Option<AttributeNode> {
        self.syntax().children().find_map(AttributeNode::cast)
    }
}

impl UnresolvedReference {
    pub fn inline_nodes(&self) -> std::vec::IntoIter<InlineNode> {
        if let Some(text) = self.syntax().children().find_map(LinkText::cast) {
            return text.inline_nodes();
        }
        if let Some(alt) = self.syntax().children().find_map(ImageAlt::cast) {
            return alt.inline_nodes();
        }
        Vec::new().into_iter()
    }
}

fn block_nodes(
    parent: &SyntaxNode,
    skip: impl Fn(SyntaxKind) -> bool,
) -> std::vec::IntoIter<BlockNode> {
    parent
        .children_with_tokens()
        .filter(|element| !skip(element.kind()))
        .map(BlockNode::cast)
        .collect::<Vec<_>>()
        .into_iter()
}

fn inline_nodes(parent: &SyntaxNode) -> std::vec::IntoIter<InlineNode> {
    parent
        .children_with_tokens()
        .filter(|element| !is_inline_marker(element.kind()))
        .map(InlineNode::cast)
        .collect::<Vec<_>>()
        .into_iter()
}

fn is_container_marker(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::BLOCK_QUOTE_MARKER
            | SyntaxKind::LINE_PREFIX
            | SyntaxKind::LIST_MARKER
            | SyntaxKind::TASK_CHECKBOX
            | SyntaxKind::ALERT_MARKER
            | SyntaxKind::DIV_FENCE_OPEN
            | SyntaxKind::DIV_FENCE_CLOSE
    )
}

fn is_inline_marker(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::EMPHASIS_MARKER
            | SyntaxKind::STRONG_MARKER
            | SyntaxKind::STRIKEOUT_MARKER
            | SyntaxKind::MARK_MARKER
            | SyntaxKind::SUPERSCRIPT_MARKER
            | SyntaxKind::SUBSCRIPT_MARKER
            | SyntaxKind::LINK_START
            | SyntaxKind::LINK_TEXT_END
            | SyntaxKind::IMAGE_LINK_START
            | SyntaxKind::IMAGE_ALT_END
    )
}

fn element_text(element: &SyntaxElement) -> String {
    match element {
        NodeOrToken::Node(node) => node.text().to_string(),
        NodeOrToken::Token(token) => token.text().to_string(),
    }
}
