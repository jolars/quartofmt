use rowan::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens
    WHITESPACE = 0,
    NEWLINE,
    TEXT,
    DivMarker,        // :::
    FrontmatterDelim, // --- or +++
    BlockQuoteMarker, // >
    ImageLinkStart,   // ![
    ListMarker,       // - + *
    CommentStart,     // <!--
    CommentEnd,       // -->
    Attribute,        // {#label} for headings, math, etc.

    // Links
    LinkStart, // [
    Link,      // [text](url)
    ImageLink, // ![alt](url)

    // Math
    InlineMathMarker, // $
    BlockMathMarker,  // $$
    InlineMath,
    MathBlock,
    MathContent,

    // Code
    CodeSpan,
    CodeFenceMarker, // ``` or ~~~
    CodeBlock,

    // Composite nodes
    ROOT,
    DOCUMENT,
    FRONTMATTER,
    FencedDiv,
    PARAGRAPH,
    BlankLine,
    BlockQuote,
    List,
    ListItem,
    Comment,

    // Headings
    Heading,
    HeadingContent,
    AtxHeadingMarker,       // leading #####
    SetextHeadingUnderline, // ===== or -----

    // LaTeX environments
    LatexCommand, // \command{...}
    LatexEnvironment,
    LatexEnvBegin, // \begin{...}
    LatexEnvEnd,   // \end{...}
    LatexEnvContent,

    // Tables
    SimpleTable,

    // Code block parts
    CodeFenceOpen,
    CodeFenceClose,
    CodeInfo,
    CodeContent,

    // Div parts
    DivFenceOpen,
    DivFenceClose,
    DivInfo,
    DivContent,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QuartoLanguage {}

impl Language for QuartoLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        unsafe { std::mem::transmute::<u16, SyntaxKind>(raw.0) }
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<QuartoLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<QuartoLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<QuartoLanguage>;
