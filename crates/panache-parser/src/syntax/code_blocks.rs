//! Code block and chunk AST node wrappers.

use std::collections::BTreeMap;

use super::{
    AstNode, ChunkInfoItem, ChunkLabel, ChunkLabelEntry, ChunkLabelSource, ChunkOption,
    ChunkOptionEntry, ChunkOptionSource, ChunkOptions, HashpipeYamlPreamble, PanacheLanguage,
    SyntaxKind, SyntaxNode, TextRange, TextSize, YamlDocument, YamlNode, YamlScalarStyle,
};

pub struct CodeBlock(SyntaxNode);

impl AstNode for CodeBlock {
    type Language = PanacheLanguage;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CODE_BLOCK
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CodeBlock {
    pub fn info(&self) -> Option<CodeInfo> {
        self.0.descendants().find_map(CodeInfo::cast)
    }

    pub fn language(&self) -> Option<String> {
        self.info()
            .and_then(|info| info.language())
            .filter(|language| !language.is_empty())
    }

    pub fn content_text(&self) -> String {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::CODE_CONTENT)
            .map(|child| child.text().to_string())
            .unwrap_or_default()
    }

    pub fn content_range(&self) -> Option<rowan::TextRange> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::CODE_CONTENT)
            .map(|child| child.text_range())
    }

    /// Source segments belonging to code rather than container framing or a
    /// hashpipe YAML preamble. Concatenating their text yields executable code;
    /// each segment retains its exact host-document range.
    pub fn code_source_segments(&self) -> Vec<CodeSourceSegment> {
        let Some(content) = self
            .0
            .children()
            .find(|child| child.kind() == SyntaxKind::CODE_CONTENT)
        else {
            return Vec::new();
        };

        content
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| token.kind() != SyntaxKind::LINE_PREFIX)
            .filter(|token| {
                !token
                    .parent()
                    .into_iter()
                    .flat_map(|parent| parent.ancestors())
                    .any(|ancestor| ancestor.kind() == SyntaxKind::HASHPIPE_YAML_PREAMBLE)
            })
            .map(|token| CodeSourceSegment {
                text: token.text().to_string(),
                range: token.text_range(),
            })
            .collect()
    }

    pub fn code_source(&self) -> String {
        self.code_source_segments()
            .into_iter()
            .map(|segment| segment.text)
            .collect()
    }

    pub fn executable_cell(&self) -> Option<ExecutableCell> {
        self.is_executable_chunk()
            .then(|| ExecutableCell(CodeBlock::cast(self.0.clone()).expect("cloned code block")))
    }

    pub fn is_executable_chunk(&self) -> bool {
        self.info().is_some_and(|info| info.is_executable())
    }

    pub fn is_display_code_block(&self) -> bool {
        self.language().is_some() && !self.is_executable_chunk()
    }

    pub fn hashpipe_yaml_preamble(&self) -> Option<HashpipeYamlPreamble> {
        self.0.descendants().find_map(HashpipeYamlPreamble::cast)
    }

    pub fn inline_chunk_option_entries(&self) -> Vec<ChunkOptionEntry> {
        self.info()
            .map(|info| {
                info.chunk_options()
                    .map(|option| {
                        ChunkOptionEntry::from_inline_option(&option, ChunkOptionSource::InlineInfo)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Chunk options from the embedded hashpipe YAML block map. The preamble's
    /// `HASHPIPE_YAML_CONTENT` carries a spliced YAML document (host-aligned
    /// ranges), so each top-level `key: value` entry becomes one option:
    /// cooked value, host-range spans, and a quoted flag from the scalar
    /// style. Non-scalar values (e.g. a `fig-subcap:` sequence) yield an entry
    /// with no value, matching the legacy option-line behavior.
    pub fn hashpipe_chunk_option_entries(&self) -> Vec<ChunkOptionEntry> {
        let Some(map) = self
            .hashpipe_yaml_preamble()
            .and_then(|preamble| {
                preamble
                    .syntax()
                    .children()
                    .find(|n| n.kind() == SyntaxKind::HASHPIPE_YAML_CONTENT)
            })
            .and_then(|content| content.children().find_map(YamlDocument::cast))
            .and_then(|doc| doc.block_map())
        else {
            return Vec::new();
        };

        map.entries()
            .map(|entry| {
                let key_scalar = entry.key().and_then(|key| key.scalar());
                let value_scalar = entry.value().and_then(|value| value.as_scalar());
                let is_quoted = value_scalar.as_ref().is_some_and(|scalar| {
                    matches!(
                        scalar.style(),
                        YamlScalarStyle::SingleQuoted | YamlScalarStyle::DoubleQuoted
                    )
                });
                ChunkOptionEntry::new(
                    entry.key_text(),
                    value_scalar.as_ref().map(|scalar| scalar.value()),
                    key_scalar.map(|scalar| scalar.text_range()),
                    value_scalar.as_ref().map(|scalar| scalar.text_range()),
                    is_quoted,
                    entry.syntax().text_range(),
                    ChunkOptionSource::HashpipeYaml,
                )
            })
            .collect()
    }

    pub fn merged_chunk_option_entries(&self) -> Vec<ChunkOptionEntry> {
        fn normalized_key(entry: &ChunkOptionEntry) -> Option<String> {
            entry.key().map(|key| key.trim().to_ascii_lowercase())
        }

        let mut seen_inline_keys = std::collections::HashSet::new();
        let mut merged = self.inline_chunk_option_entries();
        for entry in &merged {
            if let Some(key) = normalized_key(entry) {
                seen_inline_keys.insert(key);
            }
        }

        for entry in self.hashpipe_chunk_option_entries() {
            if normalized_key(&entry).is_some_and(|key| seen_inline_keys.contains(&key)) {
                continue;
            }
            merged.push(entry);
        }

        merged
    }

    pub fn inline_chunk_options_node(&self) -> Option<ChunkOptions> {
        self.info().and_then(|info| info.chunk_options_node())
    }

    pub fn chunk_label_entries(&self) -> Vec<ChunkLabelEntry> {
        let mut labels = Vec::new();

        if let Some(info) = self.info() {
            for label in info.chunk_labels() {
                let text = label.text();
                if text.is_empty() {
                    continue;
                }
                let range = label.syntax().text_range();
                labels.push(ChunkLabelEntry::new(
                    text,
                    range,
                    range,
                    ChunkLabelSource::InlineLabel,
                ));
            }
        }

        for entry in self.merged_chunk_option_entries() {
            let Some(key) = entry.key() else {
                continue;
            };
            if !key.eq_ignore_ascii_case("label") {
                continue;
            }
            let Some(value) = entry.value() else {
                continue;
            };
            if value.is_empty() {
                continue;
            }
            let value_range = entry
                .value_range()
                .unwrap_or_else(|| entry.declaration_range());
            labels.push(ChunkLabelEntry::new(
                value,
                entry.declaration_range(),
                value_range,
                ChunkLabelSource::LabelOption,
            ));
        }

        labels
    }

    pub fn chunk_labels(&self) -> Vec<String> {
        self.chunk_label_entries()
            .into_iter()
            .map(|entry| entry.value().to_string())
            .collect()
    }

    pub fn has_chunk_option_key_with_nonempty_value(&self, key_name: &str) -> bool {
        self.merged_chunk_option_entries().into_iter().any(|entry| {
            entry
                .key()
                .is_some_and(|key| key.eq_ignore_ascii_case(key_name))
                && entry.value().is_some_and(|value| !value.is_empty())
        })
    }

    pub fn has_chunk_label(&self) -> bool {
        !self.chunk_labels().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeSourceSegment {
    text: String,
    range: TextRange,
}

impl CodeSourceSegment {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn text_range(&self) -> TextRange {
        self.range
    }
}

/// Consumer-facing view of a Quarto/R Markdown executable fenced cell.
pub struct ExecutableCell(CodeBlock);

impl ExecutableCell {
    pub fn syntax(&self) -> &SyntaxNode {
        self.0.syntax()
    }

    pub fn language(&self) -> Option<String> {
        self.0.language()
    }

    pub fn text_range(&self) -> TextRange {
        self.0.syntax().text_range()
    }

    pub fn code_source_segments(&self) -> Vec<CodeSourceSegment> {
        self.0.code_source_segments()
    }

    pub fn code_source(&self) -> String {
        self.0.code_source()
    }

    pub fn code_range(&self) -> Option<TextRange> {
        let segments = self.code_source_segments();
        let first = segments.first()?.text_range();
        let last = segments.last()?.text_range();
        Some(TextRange::new(first.start(), last.end()))
    }

    pub fn identifier(&self) -> Option<(String, TextRange)> {
        self.0.info()?.chunk_items().find_map(|item| match item {
            ChunkInfoItem::Id(id) => Some(marker_payload(id.text(), id.range(), '#')),
            _ => None,
        })
    }

    pub fn classes(&self) -> Vec<(String, TextRange)> {
        self.0
            .info()
            .map(|info| {
                info.chunk_items()
                    .filter_map(|item| match item {
                        ChunkInfoItem::Class(class) => {
                            Some(marker_payload(class.text(), class.range(), '.'))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn labels(&self) -> Vec<ChunkLabelEntry> {
        let mut labels = Vec::new();
        if let Some(info) = self.0.info() {
            for label in info.chunk_labels() {
                let range = label.range();
                labels.push(ChunkLabelEntry::new(
                    label.text(),
                    range,
                    range,
                    ChunkLabelSource::InlineLabel,
                ));
            }
        }
        for option in self.option_declarations() {
            if option
                .key()
                .is_some_and(|key| key.eq_ignore_ascii_case("label"))
                && let Some(value) = option.cooked_value()
                && !value.is_empty()
            {
                labels.push(ChunkLabelEntry::new(
                    value.to_string(),
                    option.declaration_range(),
                    option
                        .value_range()
                        .unwrap_or_else(|| option.declaration_range()),
                    ChunkLabelSource::LabelOption,
                ));
            }
        }
        labels
    }

    pub fn option_declarations(&self) -> Vec<CellOptionDeclaration> {
        let mut declarations = self
            .0
            .inline_chunk_option_entries()
            .into_iter()
            .map(CellOptionDeclaration::from_inline)
            .collect::<Vec<_>>();

        if let Some(map) = self
            .0
            .hashpipe_yaml_preamble()
            .and_then(|preamble| preamble.document())
            .and_then(|document| document.block_map())
        {
            declarations.extend(map.entries().map(CellOptionDeclaration::from_hashpipe));
        }
        declarations.sort_by_key(|declaration| declaration.declaration_range().start());
        declarations
    }

    pub fn resolved_options(&self) -> Vec<ResolvedCellOption> {
        let mut grouped: BTreeMap<String, Vec<CellOptionDeclaration>> = BTreeMap::new();
        for declaration in self.option_declarations() {
            if let Some(key) = declaration.canonical_key() {
                grouped.entry(key).or_default().push(declaration);
            }
        }

        grouped
            .into_iter()
            .map(|(key, declarations)| {
                let winning_source = if declarations
                    .iter()
                    .any(|entry| entry.source() == ChunkOptionSource::InlineInfo)
                {
                    ChunkOptionSource::InlineInfo
                } else {
                    ChunkOptionSource::HashpipeYaml
                };
                let winners = declarations
                    .into_iter()
                    .filter(|entry| entry.source() == winning_source)
                    .collect::<Vec<_>>();
                let resolution = if winners.len() == 1 {
                    CellOptionResolution::Resolved(winners.into_iter().next().expect("one winner"))
                } else {
                    CellOptionResolution::Ambiguous(winners)
                };
                ResolvedCellOption { key, resolution }
            })
            .collect()
    }
}

fn marker_payload(text: String, range: TextRange, marker: char) -> (String, TextRange) {
    let marker_len = if text.starts_with(marker) {
        marker.len_utf8()
    } else {
        0
    };
    (
        text[marker_len..].to_string(),
        TextRange::new(
            range.start() + TextSize::from(marker_len as u32),
            range.end(),
        ),
    )
}

#[derive(Debug, Clone)]
pub struct CellOptionDeclaration {
    key: Option<String>,
    raw_value: Option<String>,
    cooked_value: Option<String>,
    yaml_value: Option<YamlNode>,
    key_range: Option<TextRange>,
    value_range: Option<TextRange>,
    declaration_range: TextRange,
    source: ChunkOptionSource,
    is_quoted: bool,
}

impl CellOptionDeclaration {
    fn from_inline(entry: ChunkOptionEntry) -> Self {
        Self {
            key: entry.key(),
            raw_value: entry.value(),
            cooked_value: entry.value(),
            yaml_value: None,
            key_range: entry.key_range(),
            value_range: entry.value_range(),
            declaration_range: entry.declaration_range(),
            source: entry.source(),
            is_quoted: entry.is_quoted(),
        }
    }

    fn from_hashpipe(entry: super::YamlBlockMapEntry) -> Self {
        let key_scalar = entry.key().and_then(|key| key.scalar());
        let value = entry.value();
        let yaml_value = value.as_ref().and_then(|value| value.as_node());
        let raw_value = yaml_value
            .as_ref()
            .map(|value| value.syntax().text().to_string());
        let cooked_value = yaml_value.as_ref().and_then(|value| match value {
            YamlNode::Scalar(scalar) => Some(scalar.value()),
            _ => None,
        });
        let is_quoted = yaml_value.as_ref().is_some_and(|value| {
            matches!(
                value,
                YamlNode::Scalar(scalar)
                    if matches!(
                        scalar.style(),
                        YamlScalarStyle::SingleQuoted | YamlScalarStyle::DoubleQuoted
                    )
            )
        });
        Self {
            key: entry.key_text(),
            raw_value,
            cooked_value,
            key_range: key_scalar.map(|scalar| scalar.text_range()),
            value_range: yaml_value.as_ref().map(YamlNode::text_range),
            yaml_value,
            declaration_range: entry.syntax().text_range(),
            source: ChunkOptionSource::HashpipeYaml,
            is_quoted,
        }
    }

    pub fn source(&self) -> ChunkOptionSource {
        self.source
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn canonical_key(&self) -> Option<String> {
        self.key()
            .map(|key| key.trim().to_ascii_lowercase().replace('.', "-"))
            .filter(|key| !key.is_empty())
    }

    pub fn raw_value(&self) -> Option<&str> {
        self.raw_value.as_deref()
    }

    pub fn cooked_value(&self) -> Option<&str> {
        self.cooked_value.as_deref()
    }

    pub fn yaml_value(&self) -> Option<&YamlNode> {
        self.yaml_value.as_ref()
    }

    pub fn key_range(&self) -> Option<TextRange> {
        self.key_range
    }

    pub fn value_range(&self) -> Option<TextRange> {
        self.value_range
    }

    pub fn declaration_range(&self) -> TextRange {
        self.declaration_range
    }

    pub fn is_quoted(&self) -> bool {
        self.is_quoted
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedCellOption {
    key: String,
    resolution: CellOptionResolution,
}

impl ResolvedCellOption {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn resolution(&self) -> &CellOptionResolution {
        &self.resolution
    }
}

#[derive(Debug, Clone)]
pub enum CellOptionResolution {
    Resolved(CellOptionDeclaration),
    Ambiguous(Vec<CellOptionDeclaration>),
}

pub struct CodeInfo(SyntaxNode);

impl AstNode for CodeInfo {
    type Language = PanacheLanguage;

    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::CODE_INFO
    }

    fn cast(syntax: SyntaxNode) -> Option<Self> {
        if Self::can_cast(syntax.kind()) {
            Some(Self(syntax))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CodeInfo {
    pub fn language(&self) -> Option<String> {
        self.0.children_with_tokens().find_map(|child| {
            child.into_token().and_then(|token| {
                (token.kind() == SyntaxKind::CODE_LANGUAGE).then(|| token.text().to_string())
            })
        })
    }

    pub fn is_executable(&self) -> bool {
        self.chunk_options_node().is_some()
    }

    pub fn chunk_options(&self) -> impl Iterator<Item = ChunkOption> {
        self.chunk_options_node()
            .map(|chunk_options| chunk_options.options().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn chunk_labels(&self) -> impl Iterator<Item = ChunkLabel> {
        self.chunk_options_node()
            .map(|chunk_options| chunk_options.labels().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn chunk_items(&self) -> impl Iterator<Item = ChunkInfoItem> {
        self.chunk_options_node()
            .map(|chunk_options| chunk_options.items().collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    pub fn chunk_options_node(&self) -> Option<ChunkOptions> {
        self.0.children().find_map(ChunkOptions::cast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Flavor, ParserOptions};
    use crate::parse;

    #[test]
    fn code_block_display_shortcut_wrapper() {
        let tree = parse("```python\nprint('hi')\n```\n", None);
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        assert_eq!(block.language().as_deref(), Some("python"));
        assert!(block.is_display_code_block());
        assert!(!block.is_executable_chunk());
        assert!(block.content_text().contains("print('hi')"));
    }

    #[test]
    fn code_block_executable_chunk_wrapper() {
        let config = ParserOptions {
            flavor: Flavor::Quarto,
            extensions: crate::options::Extensions::for_flavor(Flavor::Quarto),
            ..Default::default()
        };
        let tree = parse("```{r, echo=FALSE}\nx <- 1\n```\n", Some(config));
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        assert_eq!(block.language().as_deref(), Some("r"));
        assert!(block.is_executable_chunk());
        assert!(!block.is_display_code_block());

        let info = block.info().expect("code info");
        let keys: Vec<String> = info.chunk_options().filter_map(|opt| opt.key()).collect();
        assert!(keys.contains(&"echo".to_string()));
    }

    #[test]
    fn code_block_hashpipe_preamble_wrapper() {
        let config = ParserOptions {
            flavor: Flavor::Quarto,
            extensions: crate::options::Extensions::for_flavor(Flavor::Quarto),
            ..Default::default()
        };
        let tree = parse(
            "```{python}\n#| echo: false\nprint('hi')\n```\n",
            Some(config),
        );
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        assert!(block.hashpipe_yaml_preamble().is_some());
    }

    #[test]
    fn code_block_collects_chunk_labels_and_options() {
        let config = ParserOptions {
            flavor: Flavor::Quarto,
            extensions: crate::options::Extensions::for_flavor(Flavor::Quarto),
            ..Default::default()
        };
        let tree = parse(
            "```{r chunk_inline, echo=FALSE}\n#| label: chunk_hashpipe\n#| fig-cap: \"Caption\"\n1 + 1\n```\n",
            Some(config),
        );
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        let labels = block.chunk_labels();
        assert!(labels.iter().any(|label| label == "chunk_inline"));
        assert!(labels.iter().any(|label| label == "chunk_hashpipe"));
        assert!(block.has_chunk_label());
        assert!(block.has_chunk_option_key_with_nonempty_value("fig-cap"));
    }

    #[test]
    fn merged_chunk_options_prefer_inline_over_hashpipe() {
        let config = ParserOptions {
            flavor: Flavor::Quarto,
            extensions: crate::options::Extensions::for_flavor(Flavor::Quarto),
            ..Default::default()
        };
        let tree = parse(
            "```{r, label=inline, echo=true}\n#| label: hashpipe\n#| echo: false\n1 + 1\n```\n",
            Some(config),
        );
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        let merged = block.merged_chunk_option_entries();
        let mut labels = merged
            .iter()
            .filter_map(|entry| {
                let key = entry.key()?;
                key.eq_ignore_ascii_case("label").then(|| {
                    (
                        entry.value().unwrap_or_default(),
                        entry.source() == ChunkOptionSource::InlineInfo,
                    )
                })
            })
            .collect::<Vec<_>>();
        labels.sort();
        assert_eq!(labels, vec![("inline".to_string(), true)]);

        let mut echoes = merged
            .iter()
            .filter_map(|entry| {
                let key = entry.key()?;
                key.eq_ignore_ascii_case("echo")
                    .then(|| entry.value().unwrap_or_default())
            })
            .collect::<Vec<_>>();
        echoes.sort();
        assert_eq!(echoes, vec!["true".to_string()]);
    }

    #[test]
    fn chunk_label_entries_include_ranges() {
        let config = ParserOptions {
            flavor: Flavor::Quarto,
            extensions: crate::options::Extensions::for_flavor(Flavor::Quarto),
            ..Default::default()
        };
        let tree = parse("```{r chunk_a, label=chunk_b}\n1 + 1\n```\n", Some(config));
        let block = tree
            .descendants()
            .find_map(CodeBlock::cast)
            .expect("code block");

        let labels = block.chunk_label_entries();
        assert_eq!(labels.len(), 2);
        assert!(labels.iter().any(|entry| {
            entry.value() == "chunk_a"
                && entry.source() == ChunkLabelSource::InlineLabel
                && !entry.value_range().is_empty()
        }));
        assert!(labels.iter().any(|entry| {
            entry.value() == "chunk_b"
                && entry.source() == ChunkLabelSource::LabelOption
                && !entry.value_range().is_empty()
        }));
    }
}
