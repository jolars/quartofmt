use rowan::{TextRange, TextSize};

use crate::linter::diagnostics::{Diagnostic, DiagnosticNoteKind, Location};
use crate::linter::rules::{DiagnosticCode, LintContext, Requirement, Rule, RuleMeta};
use crate::syntax::{AstNode, BlockNode, ListItem, SyntaxKind, SyntaxToken};

pub struct UnspacedListMarkerRule;

impl Rule for UnspacedListMarkerRule {
    fn name(&self) -> &str {
        "unspaced-list-marker"
    }

    fn metadata(&self) -> RuleMeta {
        RuleMeta {
            name: "unspaced-list-marker",
            default_on: true,
            requires: Requirement::Always,
            auto_fix: false,
            codes: const { &[DiagnosticCode::warning("unspaced-list-marker")] },
        }
    }

    fn wants_text_tokens(&self) -> bool {
        true
    }

    fn check(&self, cx: &LintContext) -> Vec<Diagnostic> {
        cx.text_tokens()
            .iter()
            .filter_map(|token| {
                let range = unspaced_marker_range(token)?;
                let parent = token.parent()?;
                // Inline spans and code can contain literal examples of this syntax.
                if !matches!(
                    BlockNode::cast(parent.clone().into()),
                    BlockNode::Paragraph(_) | BlockNode::Plain(_)
                ) || parent.ancestors().find_map(ListItem::cast).is_none()
                    || !starts_source_line(token)
                {
                    return None;
                }

                Some(
                    Diagnostic::warning(
                        Location::from_range(range, cx.input),
                        self.name(),
                        "Missing whitespace after '-': this line is paragraph text",
                    )
                    .with_note(
                        DiagnosticNoteKind::Help,
                        "If you intended a list item, add a space after '-' and check its \
                         indentation. Escape the hyphen as '\\-' if it is literal text",
                    ),
                )
            })
            .collect()
    }
}

fn unspaced_marker_range(token: &SyntaxToken) -> Option<TextRange> {
    let text = token.text().trim_start_matches([' ', '\t']);
    let next = token.next_sibling_or_token();
    let matches = match text {
        "-" => next.as_ref().is_some_and(|element| {
            element.as_token().is_some_and(|next| {
                (next.kind() == SyntaxKind::HARD_LINE_BREAK && next.text().starts_with('\\'))
                    || (next.kind() == SyntaxKind::ESCAPED_CHAR
                        && next.text() == "\\\r"
                        && next
                            .next_sibling_or_token()
                            .is_some_and(|newline| newline.kind() == SyntaxKind::NEWLINE))
            })
        }),
        // At the end of a CommonMark block or with escaped line breaks disabled,
        // the backslash remains text. It still cannot form a list marker.
        "-\\" => next
            .as_ref()
            .is_none_or(|next| next.kind() == SyntaxKind::NEWLINE),
        _ => false,
    };
    if !matches {
        return None;
    }
    let indent = token.text().len() - text.len();
    let start = token.text_range().start() + TextSize::from(indent as u32);
    Some(TextRange::new(start, start + TextSize::from(2)))
}

fn starts_source_line(token: &SyntaxToken) -> bool {
    // Looking past structural prefixes also handles lists inside block quotes
    // and paragraphs after a blank line, without mistaking `- -\` for a new line.
    let previous =
        std::iter::successors(token.prev_token(), SyntaxToken::prev_token).find(|token| {
            !matches!(
                token.kind(),
                SyntaxKind::WHITESPACE | SyntaxKind::LINE_PREFIX
            )
        });
    previous.is_some_and(|token| {
        matches!(
            token.kind(),
            SyntaxKind::NEWLINE | SyntaxKind::HARD_LINE_BREAK | SyntaxKind::BLANK_LINE
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Flavor};

    fn lint_with(input: &str, config: Config) -> Vec<Diagnostic> {
        let tree = crate::parser::parse(input, Some(config.clone()));
        UnspacedListMarkerRule.check_tree(&tree, input, &config, None)
    }

    fn lint(input: &str) -> Vec<Diagnostic> {
        lint_with(input, Config::default())
    }

    #[test]
    fn flags_unspaced_marker_across_flavors() {
        for flavor in [
            Flavor::Pandoc,
            Flavor::Quarto,
            Flavor::RMarkdown,
            Flavor::CommonMark,
            Flavor::Gfm,
            Flavor::MultiMarkdown,
            Flavor::Mdsvex,
            Flavor::Myst,
        ] {
            for input in [
                "- first\n  -\\\n- second\n",
                "- first\n  -\\\n  continued\n",
                "- first\r\n  -\\\r\n- second\r\n",
                "- first\n  -\\",
            ] {
                let config = Config {
                    flavor,
                    ..Config::default()
                };
                let diagnostics = lint_with(input, config);
                assert_eq!(diagnostics.len(), 1, "{flavor:?}: {input:?}");
                assert_eq!(&input[diagnostics[0].location.range], "-\\");
                assert!(diagnostics[0].fix.is_none());
                assert!(diagnostics[0].notes[0].message.contains("add a space"));
            }
        }
    }

    #[test]
    fn flags_markers_in_nested_and_loose_containers() {
        for input in [
            "- first\n  - nested\n    -\\\n- second\n",
            "- first\n\n  -\\\n\n- second\n",
            "> - first\n>   -\\\n> - second\n",
            "- first\n\n  > text\n  > -\\\n",
            "1. first\n   -\\\n2. second\n",
            "- first\n\t-\\\n- second\n",
        ] {
            let diagnostics = lint(input);
            assert_eq!(diagnostics.len(), 1, "{input:?}");
            assert_eq!(&input[diagnostics[0].location.range], "-\\");
        }
    }

    #[test]
    fn flags_each_consecutive_marker() {
        let input = "- first\n  -\\\n  -\\\n- second\n";
        let diagnostics = lint(input);
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics[0].location.line, 2);
        assert_eq!(diagnostics[1].location.line, 3);
    }

    #[test]
    fn does_not_require_escaped_line_breaks() {
        let config = toml::from_str::<Config>(
            "flavor = 'pandoc'\n[extensions]\nescaped-line-breaks = false\n",
        )
        .unwrap();
        assert_eq!(lint_with("- first\n  -\\\n- second\n", config).len(), 1);
    }

    #[test]
    fn ignores_ordinary_prose_and_valid_markers() {
        for input in [
            "text\n-\\\n",
            "> text\n> -\\\n",
            "- **attention:** possible\n  **platform**\n  **report**\n",
            "- first\n  - \\\n- second\n",
            "- first\n  - child\n- second\n",
            "- first\n  \\-\\\n- second\n",
            "- first -\\\n  continued\n",
            "- -\\\n  continued\n",
            "- first\n  -\\path\n- second\n",
            "- first\n  -\\\\\n- second\n",
        ] {
            assert!(lint(input).is_empty(), "{input:?}");
        }
    }

    #[test]
    fn ignores_code_and_inline_spans_inside_lists() {
        for input in [
            "- first\n\n  ```text\n  -\\\n  ```\n",
            "- first\n\n      -\\\n",
            "- first\n  `-\\`\n",
            "- *first\n  -\\\n  last*\n",
            "- [first\n  -\\\n  last](https://example.org)\n",
        ] {
            assert!(lint(input).is_empty(), "{input:?}");
        }
    }
}
