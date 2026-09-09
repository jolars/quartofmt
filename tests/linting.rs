//! Integration tests for linting rules.
//!
//! Test files are stored in `tests/linting/*.md` and tested with direct assertions.

use panache::{Config, linter::lint};
use std::fs;
use std::path::Path;

fn lint_file(filename: &str) -> Vec<panache::linter::Diagnostic> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("linting")
        .join(filename);

    let input = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {}", filename));

    let config = Config::default();
    let tree = panache::parse(&input, Some(config.clone()));
    lint(&tree, &input, &config)
}

fn lint_file_with_config(filename: &str, config_toml: &str) -> Vec<panache::linter::Diagnostic> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("linting")
        .join(filename);
    let input = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read {}", filename));
    let config = toml::from_str::<Config>(config_toml).expect("valid config");
    let tree = panache::parse(&input, Some(config.clone()));
    lint(&tree, &input, &config)
}

#[test]
fn test_ignore_directives() {
    let diagnostics = lint_file("ignore_directives.md");
    let hierarchy_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "heading-hierarchy")
        .collect();

    // Should find 1 heading hierarchy issue:
    // Line 3: Skip from h1 to h4
    // The h5 on line 9 is in an ignore region and won't be reported
    // Note: The rule still sees headings in ignore regions when tracking context,
    // so h2 after h5 doesn't violate because prev_level is updated to h5
    assert_eq!(
        hierarchy_issues.len(),
        1,
        "Should find 1 heading hierarchy issue"
    );

    // Check that we found the right violation
    assert_eq!(
        hierarchy_issues[0].location.line, 3,
        "Should warn about h4 at line 3"
    );
}

#[test]
fn test_duplicate_references() {
    let diagnostics = lint_file("duplicate_references.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(dup.len(), 1, "Should find exactly 1 duplicate");
    assert!(dup[0].message.contains("[ref1]"));
    assert_eq!(dup[0].location.line, 10);
}

#[test]
fn test_duplicate_case_insensitive() {
    let diagnostics = lint_file("duplicate_case_insensitive.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(dup.len(), 2, "Should find 2 duplicates (case-insensitive)");
    assert!(dup[0].message.contains("[myref]"));
    assert!(dup[1].message.contains("[MYREF]"));
    assert_eq!(dup[0].location.line, 6);
    assert_eq!(dup[1].location.line, 7);
}

#[test]
fn test_duplicate_yaml_anchor() {
    let diagnostics = lint_file("duplicate_yaml_anchor.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-yaml-anchor")
        .collect();

    assert_eq!(dup.len(), 1, "Should find exactly 1 duplicate anchor");
    assert!(dup[0].message.contains("`&defaults`"));
    // Flags the second declaration (line 4), not the first (line 2).
    assert_eq!(dup[0].location.line, 4);
}

#[test]
fn test_unused_yaml_anchor() {
    let quarto = "flavor = \"quarto\"\n";
    let diagnostics = lint_file_with_config("unused_yaml_anchor.qmd", quarto);
    let unused: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-yaml-anchor")
        .collect();

    // Both the frontmatter `&brand` and the hashpipe `&unused-opts` are unused.
    assert_eq!(unused.len(), 2, "Should find 2 unused anchors");
    assert!(unused.iter().any(|d| d.message.contains("`&brand`")));
    assert!(unused.iter().any(|d| d.message.contains("`&unused-opts`")));
}

#[test]
fn test_duplicate_footnotes() {
    let diagnostics = lint_file("duplicate_footnotes.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(dup.len(), 2, "Should find 2 duplicate footnotes");
    assert!(dup.iter().any(|d| d.message.contains("[^1]")));
    assert!(dup.iter().any(|d| d.message.contains("[^note]")));
}

#[test]
fn test_link_text_is_url() {
    let diagnostics = lint_file("link_text_is_url.md");
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "link-text-is-url")
        .collect();

    // The fixture has six bracket-shape candidates, but only the first should fire:
    // - line 5 (plain match)               → fires
    // - line 9 (trailing-slash mismatch)   → skip (destination changes)
    // - line 13 (scheme-less /docs/intro)  → skip (autolink validator rejects)
    // - line 17 (title present)            → skip
    // - line 21 (formatted text)           → skip
    // - line 25 (reference-style link)     → skip (out of scope)
    assert_eq!(
        hits.len(),
        1,
        "expected only the plain match to fire, got: {:?}",
        hits.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert_eq!(hits[0].location.line, 5);
    let fix = hits[0].fix.as_ref().expect("autofix");
    assert_eq!(fix.edits.len(), 1);
    assert_eq!(fix.edits[0].replacement, "<https://example.com/>");
}

#[test]
fn test_empty_list_item() {
    let diagnostics = lint_file("empty_list_item.md");
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "empty-list-item")
        .collect();

    // Expect six hits:
    //   1. bare `-` between two bullets (line 6)
    //   2. bare `1.` opening the ordered list (line 11)
    //   3. bare `3.` at the end of the ordered list (line 13)
    //   4. bare `-` that became a Setext H2 underline (line 18)
    //   5. marker followed by only trailing whitespace (line 28)
    //   6 ... nothing else; the H1 (===) case and the clean lists below stay quiet.
    let lines: Vec<usize> = hits.iter().map(|d| d.location.line).collect();
    assert_eq!(
        hits.len(),
        5,
        "unexpected hit count, lines: {:?}, messages: {:?}",
        lines,
        hits.iter().map(|d| &d.message).collect::<Vec<_>>(),
    );
    assert_eq!(lines, vec![6, 11, 13, 18, 28]);

    let setext_hit = hits.iter().find(|d| d.location.line == 18).unwrap();
    assert!(
        setext_hit.message.contains("Setext"),
        "line 18 should mention the Setext consequence, got {:?}",
        setext_hit.message,
    );

    assert!(
        hits.iter().all(|d| d.fix.is_none()),
        "empty-list-item should not ship an autofix",
    );
}

#[test]
fn test_no_duplicates() {
    let diagnostics = lint_file("no_duplicates.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(dup.len(), 0, "Clean file should have no duplicates");
}

#[test]
fn test_chunk_label_and_heading_id_can_share_label() {
    let diagnostics = lint_file_with_config(
        "chunk_label_and_heading_id_same_label.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert!(
        dup.is_empty(),
        "Heading IDs and chunk labels should not be treated as duplicate cross-reference labels"
    );
}

#[test]
fn test_duplicate_bookdown_crossref_labels() {
    // Regression: `duplicate_references` now consumes the document's real config
    // extensions (via the memoized symbol index) instead of a hardcoded
    // `Extensions::default()`. Bookdown text declarations `(\#eq:...)` are gated
    // on `bookdown_equation_references`, which RMarkdown enables, so a duplicated
    // bookdown crossref label is now flagged where it previously slipped through.
    let diagnostics = lint_file_with_config(
        "duplicate_bookdown_crossref.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(
        dup.len(),
        1,
        "duplicated bookdown `(\\#eq:dup)` should be flagged once under rmarkdown",
    );
    assert!(
        dup[0].message.contains("eq:dup"),
        "diagnostic should name the duplicated label, got: {}",
        dup[0].message,
    );
}

#[test]
fn test_whitespace_normalization() {
    let diagnostics = lint_file("whitespace_normalization.md");
    let dup: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "duplicate-reference-labels")
        .collect();

    assert_eq!(
        dup.len(),
        2,
        "Whitespace should be normalized - all 3 labels match"
    );
    // All reference the first definition on line 5
    assert!(dup[0].message.contains("first defined at line 5"));
    assert!(dup[1].message.contains("first defined at line 5"));
}

#[test]
fn test_undefined_anchor() {
    let diagnostics = lint_file("undefined_anchor.md");
    let anchors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-anchor")
        .collect();

    assert_eq!(anchors.len(), 1, "Should flag 1 unresolvable anchor");
    assert!(anchors[0].message.contains("#reel"));
    assert_eq!(anchors[0].location.line, 3);
}

#[test]
fn test_citation_ref_anchor() {
    // Pandoc renders bibliography entries with id="ref-<citekey>"; links of
    // the shape [text](#ref-citekey) override the citation's display text.
    // See https://github.com/jolars/panache/discussions/289 and
    // https://github.com/jgm/pandoc/issues/11657.
    let diagnostics = lint_file("citation_ref_anchor.md");
    let anchors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-anchor")
        .collect();

    assert_eq!(
        anchors.len(),
        1,
        "ref-<citekey> anchors should resolve for cited keys, got {:?}",
        anchors
    );
    assert!(anchors[0].message.contains("#ref-missing"));
}

#[test]
fn test_missing_reference_targets() {
    let diagnostics = lint_file("missing_references.md");
    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();
    let missing_footnote: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-footnote-id")
        .collect();

    assert_eq!(
        missing_ref.len(),
        1,
        "Should flag 1 missing reference label"
    );
    assert_eq!(missing_footnote.len(), 1, "Should flag 1 missing footnote");
    assert!(missing_ref[0].message.contains("[missing]"));
    assert!(missing_footnote[0].message.contains("[^missing-note]"));
}

#[test]
fn test_missing_reference_targets_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "missing_references.md",
        r#"
[lint.rules]
undefined-references = false
"#,
    );

    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();
    let missing_footnote: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-footnote-id")
        .collect();

    assert!(missing_ref.is_empty());
    assert!(missing_footnote.is_empty());
}

#[test]
fn test_math_delimiter_diagnostics() {
    // `math-syntax` requires a tex-math extension; the Pandoc flavor enables it.
    let diagnostics = lint_file_with_config("math_delimiters.md", "flavor = \"pandoc\"\n");
    let codes: Vec<&str> = diagnostics.iter().map(|d| d.code.as_str()).collect();
    // Unclosed `\left(` in the display block, stray `\right)` in the inline span,
    // and an unescaped `$` inside display math.
    assert!(
        codes.contains(&"math-unclosed-delimiter"),
        "expected math-unclosed-delimiter, got {codes:?}"
    );
    assert!(
        codes.contains(&"math-unexpected-right"),
        "expected math-unexpected-right, got {codes:?}"
    );
    assert!(
        codes.contains(&"math-unexpected-dollar"),
        "expected math-unexpected-dollar, got {codes:?}"
    );
}

#[test]
fn test_math_syntax_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "math_delimiters.md",
        r#"
flavor = "pandoc"

[lint.rules]
math-syntax = false
"#,
    );
    assert!(
        !diagnostics.iter().any(|d| d.code.starts_with("math-")),
        "math-syntax diagnostics should be disabled, got {diagnostics:#?}"
    );
}

#[test]
fn test_blank_line_in_display_math() {
    let diagnostics =
        lint_file_with_config("blank_line_in_display_math.qmd", "flavor = \"quarto\"\n");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "blank-line-in-display-math")
        .collect();

    assert_eq!(issues.len(), 1, "expected 1 diagnostic, got {issues:#?}");
    assert_eq!(issues[0].location.line, 3);
    assert_eq!(issues[0].location.column, 1);
    assert_eq!(u32::from(issues[0].location.range.len()), 2);
    assert!(issues[0].fix.is_none());
}

#[test]
fn test_blank_line_in_display_math_requires_dollar_math() {
    let diagnostics = lint_file_with_config(
        "blank_line_in_display_math.qmd",
        r#"
flavor = "commonmark"

[extensions]
tex-math-single-backslash = true
"#,
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "blank-line-in-display-math"),
        "rule should be gated off without extensions.tex-math-dollars"
    );
}

#[test]
fn test_inline_math_line_break() {
    let diagnostics = lint_file_with_config("inline_math_line_break.qmd", "flavor = \"quarto\"\n");
    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "inline-math-line-break")
        .collect();

    assert_eq!(
        hits.len(),
        1,
        "expected one top-level inline break: {hits:#?}"
    );
    assert_eq!(hits[0].location.line, 3);
    assert_eq!(hits[0].location.column, 32);
    assert_eq!(u32::from(hits[0].location.range.len()), 2);
    assert!(hits[0].notes[0].message.contains(r"use `\mathrm`"));
    assert!(hits[0].fix.is_none());
}

#[test]
fn test_unused_definitions() {
    let diagnostics = lint_file("unused_definitions.md");
    let unused_labels: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-definition-label")
        .collect();
    let unused_footnotes: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-footnote-id")
        .collect();

    assert_eq!(
        unused_labels.len(),
        1,
        "Should flag one unused reference label"
    );
    assert_eq!(unused_footnotes.len(), 1, "Should flag one unused footnote");
    assert!(unused_labels[0].message.contains("[unusedlabel]"));
    assert!(unused_footnotes[0].message.contains("[^2]"));
}

#[test]
fn test_unused_definitions_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "unused_definitions.md",
        r#"
[lint.rules]
unused-definitions = false
"#,
    );

    let unused_labels: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-definition-label")
        .collect();
    let unused_footnotes: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-footnote-id")
        .collect();

    assert!(unused_labels.is_empty());
    assert!(unused_footnotes.is_empty());
}

#[test]
fn test_bookdown_chunk_crossref_is_resolved() {
    let diagnostics = lint_file_with_config(
        "bookdown_chunk_crossref.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );

    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();

    assert!(
        missing_ref.is_empty(),
        "Bookdown chunk cross-reference should resolve against chunk labels"
    );
}

#[test]
fn test_bookdown_theorem_crossref_is_resolved() {
    let diagnostics = lint_file_with_config(
        "bookdown_theorem_crossref.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );

    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();

    assert!(
        missing_ref.is_empty(),
        "Bookdown theorem cross-reference should resolve against fenced div id"
    );
}

#[test]
fn test_bookdown_equation_crossref_is_resolved() {
    let diagnostics = lint_file_with_config(
        "bookdown_equation_crossref.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );

    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();

    assert!(
        missing_ref.is_empty(),
        "Bookdown equation cross-reference should resolve against equation labels"
    );
}

#[test]
fn test_bookdown_equation_crossref_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "bookdown_equation_crossref.Rmd",
        r#"
flavor = "rmarkdown"

[extensions]
bookdown-equation-references = false
"#,
    );

    let missing_ref: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "undefined-reference-label")
        .collect();

    assert_eq!(
        missing_ref.len(),
        2,
        "Disabling bookdown equation references should restore unresolved eq diagnostics"
    );
}

#[test]
fn test_chunk_label_spaces() {
    let diagnostics = lint_file_with_config(
        "chunk_label_spaces.md",
        r#"
flavor = "quarto"
"#,
    );
    let label_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "chunk-label-spaces")
        .collect();

    assert_eq!(
        label_issues.len(),
        2,
        "Should flag labels containing spaces"
    );
    assert!(label_issues[0].message.contains("several words"));
    assert!(label_issues[1].message.contains("another label"));
}

#[test]
fn test_chunk_label_spaces_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "chunk_label_spaces.md",
        r#"
flavor = "quarto"

[lint.rules]
chunk-label-spaces = false
"#,
    );

    let label_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "chunk-label-spaces")
        .collect();
    assert!(label_issues.is_empty());
}

#[test]
fn test_missing_chunk_labels() {
    let diagnostics = lint_file_with_config(
        "missing_chunk_labels.md",
        r#"
flavor = "quarto"
"#,
    );
    let missing: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "missing-chunk-labels")
        .collect();
    assert_eq!(
        missing.len(),
        1,
        "Should flag only unlabeled executable chunks"
    );
}

#[test]
fn test_missing_chunk_labels_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "missing_chunk_labels.md",
        r#"
flavor = "quarto"

[lint.rules]
missing-chunk-labels = false
"#,
    );
    let missing: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "missing-chunk-labels")
        .collect();
    assert!(missing.is_empty());
}

#[test]
fn test_missing_figure_crossref_captions_quarto_is_not_flagged() {
    let diagnostics = lint_file_with_config(
        "missing_figure_crossref_captions.qmd",
        r#"
flavor = "quarto"
"#,
    );
    let caption_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "figure-crossref-captions")
        .collect();

    assert!(
        caption_issues.is_empty(),
        "Quarto figure crossrefs should not require fig-cap"
    );
}

#[test]
fn test_missing_figure_crossref_captions_bookdown() {
    let diagnostics = lint_file_with_config(
        "missing_figure_crossref_captions.Rmd",
        r#"
flavor = "rmarkdown"
"#,
    );
    let caption_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "figure-crossref-captions")
        .collect();

    assert_eq!(
        caption_issues.len(),
        1,
        "Should flag one bookdown figure crossref with missing caption"
    );
    assert!(caption_issues[0].message.contains("@fig:a-label"));
}

#[test]
fn test_missing_figure_crossref_captions_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "missing_figure_crossref_captions.Rmd",
        r#"
flavor = "rmarkdown"

[lint.rules]
figure-crossref-captions = false
"#,
    );
    let caption_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "figure-crossref-captions")
        .collect();
    assert!(caption_issues.is_empty());
}

#[test]
fn test_unknown_emoji_alias() {
    let diagnostics = lint_file_with_config(
        "emoji_aliases.md",
        r#"
[extensions]
emoji = true
"#,
    );

    let emoji_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unknown-emoji-alias")
        .collect();
    assert_eq!(emoji_issues.len(), 1, "Should flag one unknown emoji alias");
    assert!(emoji_issues[0].message.contains(":not-a-real-emoji:"));
}

#[test]
fn test_unknown_emoji_alias_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "emoji_aliases.md",
        r#"
[extensions]
emoji = true

[lint.rules]
unknown-emoji-alias = false
"#,
    );

    let emoji_issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unknown-emoji-alias")
        .collect();
    assert!(emoji_issues.is_empty());
}

#[test]
fn test_unused_definitions_resolved_across_project_files() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let root = temp_dir.path();
    let doc1 = root.join("1-one.Rmd");
    let doc2 = root.join("2-two.Rmd");

    fs::write(root.join("_bookdown.yml"), "").unwrap();
    fs::write(&doc1, "[shared]: https://example.com\n").unwrap();
    fs::write(&doc2, "See [x][shared].\n").unwrap();

    let input = fs::read_to_string(&doc1).unwrap();
    let config = toml::from_str::<Config>("flavor = \"rmarkdown\"").expect("valid config");
    let tree = panache::parse(&input, Some(config.clone()));
    let metadata = panache::metadata::extract_project_metadata(&tree, &doc1).ok();
    let diagnostics =
        panache::linter::lint_with_metadata(&tree, &input, &config, metadata.as_ref());

    let unused_labels: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unused-definition-label")
        .collect();
    assert!(
        unused_labels.is_empty(),
        "Definition used in sibling project document should not be flagged unused"
    );
}

#[test]
fn test_html_entities_default_on() {
    let diagnostics = lint_file("html_entities.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "html-entities")
        .collect();

    assert_eq!(
        issues.len(),
        3,
        "expected exactly 3 html-entities diagnostics (typo + missing-semi + near-miss), got {:?}",
        issues.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let typo = issues
        .iter()
        .find(|d| d.message.contains("&ellips;"))
        .expect("typo diagnostic for &ellips;");
    assert_eq!(typo.location.line, 1);

    let missing_semi = issues
        .iter()
        .find(|d| d.message.contains("&numero"))
        .expect("missing-semicolon diagnostic for &numero");
    assert_eq!(missing_semi.location.line, 3);
    assert!(missing_semi.message.contains("missing"));

    let near_miss = issues
        .iter()
        .find(|d| d.message.contains("&hellp"))
        .expect("near-miss diagnostic for &hellp");
    assert_eq!(near_miss.location.line, 5);
    assert!(
        near_miss
            .notes
            .iter()
            .any(|n| n.message.contains("&hellip;"))
    );
}

#[test]
fn test_html_entities_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "html_entities.md",
        r#"
[lint.rules]
html-entities = false
"#,
    );

    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "html-entities")
        .collect();
    assert!(issues.is_empty());
}

#[test]
fn test_adjacent_footnote_refs() {
    let diagnostics = lint_file("adjacent_footnote_refs.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "adjacent-footnote-refs")
        .collect();

    // One gap between [^a][^b] and two gaps in [^e][^f][^g] = 3 total.
    assert_eq!(issues.len(), 3, "expected 3 diagnostics, got {:?}", issues);
    for diag in &issues {
        let fix = diag.fix.as_ref().expect("rule provides an auto-fix");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].replacement, " ");
    }
}

#[test]
fn test_blank_line_in_inline_footnote() {
    let diagnostics = lint_file("blank_line_in_inline_footnote.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "blank-line-in-inline-footnote")
        .collect();

    assert_eq!(issues.len(), 1, "expected 1 diagnostic, got {issues:#?}");
    assert_eq!(issues[0].location.line, 3);
    assert_eq!(issues[0].location.column, 33);
    assert_eq!(u32::from(issues[0].location.range.len()), 2);
    assert!(issues[0].fix.is_none());
}

#[test]
fn test_blank_line_in_inline_footnote_needs_inline_footnotes() {
    let diagnostics = lint_file_with_config(
        "blank_line_in_inline_footnote.md",
        "flavor = \"commonmark\"\n",
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "blank-line-in-inline-footnote"),
        "rule should be gated off without extensions.inline-footnotes"
    );
}

#[test]
fn test_footnote_ref_in_footnote_def() {
    let diagnostics = lint_file("footnote_ref_in_footnote_def.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "footnote-ref-in-footnote-def")
        .collect();

    // Expected hits inside def bodies:
    //   1. `[^b]` in [^a]:'s body
    //   2. `[^c]` inside **bold** in [^b]:'s body
    //   3. `[^d]` inside ~~strike~~ in [^b]:'s body
    //   4. `[^e]` inside [link]() in [^b]:'s body
    //   5. `[^g]` inside the nested blockquote in [^d]:'s body
    //   6. `[^h]` inside the nested list in [^d]:'s body
    // Non-hits: `[^f]` in a code span, `[@key]` citations, outer refs.
    assert_eq!(issues.len(), 6, "expected 6 diagnostics, got {:?}", issues);

    for diag in &issues {
        assert!(diag.fix.is_none(), "rule must not auto-fix");
        assert!(
            diag.notes.iter().any(|n| n.message.contains("nest")),
            "expected help note explaining footnotes don't nest"
        );
        assert!(diag.message.contains("pandoc"));
    }
}

#[test]
fn test_footnote_ref_in_footnote_def_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "footnote_ref_in_footnote_def.md",
        r#"
[lint.rules]
footnote-ref-in-footnote-def = false
"#,
    );

    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "footnote-ref-in-footnote-def")
        .collect();
    assert!(issues.is_empty());
}

#[test]
fn test_crossref_as_link_target() {
    let diagnostics = lint_file("crossref_as_link_target.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "crossref-as-link-target")
        .collect();

    // Four offenders: inline link, citation-key link, image link, wrapped link.
    assert_eq!(issues.len(), 4, "expected 4 diagnostics, got {:?}", issues);

    for diag in &issues {
        let fix = diag.fix.as_ref().expect("rule provides an auto-fix");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].replacement, "#");
        let r = fix.edits[0].range;
        let span: usize = (r.end() - r.start()).into();
        assert_eq!(span, 1, "fix span must target exactly the '@' byte");
    }
}

#[test]
fn test_crossref_as_link_target_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "crossref_as_link_target.md",
        r#"
[lint.rules]
crossref-as-link-target = false
"#,
    );

    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "crossref-as-link-target")
        .collect();
    assert!(issues.is_empty());
}

#[test]
fn test_heading_eaten_attrs() {
    let diagnostics = lint_file("heading_eaten_attrs.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "heading-eaten-attrs")
        .collect();

    // Line 1: one comment after the (eaten) attr block.
    // Line 9: two comments around the (eaten) attr block.
    assert_eq!(issues.len(), 3, "expected 3 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![1, 9, 9]);
    assert!(issues.iter().all(|d| d.message.contains("literal")));
    assert!(issues.iter().all(|d| d.fix.is_none()));
}

#[test]
fn test_heading_eaten_attrs_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "heading_eaten_attrs.md",
        r#"
[lint.rules]
heading-eaten-attrs = false
"#,
    );

    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "heading-eaten-attrs")
        .collect();
    assert!(issues.is_empty());
}

#[test]
fn test_heading_strip_comments_residue_off_by_default() {
    let diagnostics = lint_file("heading_strip_comments_residue.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "heading-strip-comments-residue")
        .collect();
    assert!(
        issues.is_empty(),
        "expected no diagnostics by default, got {:#?}",
        issues
    );
}

#[test]
fn test_heading_strip_comments_residue_when_enabled() {
    let diagnostics = lint_file_with_config(
        "heading_strip_comments_residue.md",
        r#"
[lint.rules]
heading-strip-comments-residue = true
"#,
    );
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "heading-strip-comments-residue")
        .collect();

    // Only line 1 should fire: line 7's attrs are eaten (sibling rule), so
    // there are no real attrs adjacent to the comment.
    assert_eq!(issues.len(), 1, "expected 1 diagnostic, got {:#?}", issues);
    assert_eq!(issues[0].location.line, 1);
    assert!(issues[0].message.contains("--strip-comments"));
    assert!(issues[0].fix.is_none());
}

#[test]
fn test_stray_fenced_div_markers() {
    let diagnostics = lint_file("stray_fenced_div_markers.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "stray-fenced-div-markers")
        .collect();

    assert_eq!(issues.len(), 4, "expected 4 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![9, 19, 23, 26]);
    assert!(issues.iter().all(|d| d.fix.is_none()));
    assert!(
        issues[0].message.contains(":::"),
        "message should mention the marker"
    );
}

#[test]
fn test_swallowed_list_marker() {
    let diagnostics = lint_file("swallowed_list_marker.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "swallowed-list-marker")
        .collect();

    assert_eq!(issues.len(), 4, "expected 4 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![4, 13, 17, 20]);
    assert!(issues.iter().all(|d| d.fix.is_none()));
    assert!(
        issues[0].message.contains("list marker"),
        "message should name the failure mode"
    );
}

#[test]
fn test_swallowed_list_marker_commonmark() {
    // Under CommonMark bullets and `1.` interrupt the paragraph and become
    // real lists, so only the run starting at `2.` is still swallowed.
    let diagnostics = lint_file_with_config(
        "swallowed_list_marker_commonmark.md",
        "flavor = \"commonmark\"\n",
    );
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "swallowed-list-marker")
        .collect();

    assert_eq!(issues.len(), 1, "expected 1 diagnostic, got {:#?}", issues);
    assert_eq!(issues[0].location.line, 8);
    assert!(
        issues[0]
            .notes
            .iter()
            .any(|n| n.message.contains("start at 1")),
        "help should explain the CommonMark rule, got {:#?}",
        issues[0].notes
    );
}

#[test]
fn test_unspaced_list_marker() {
    let input = include_str!("linting/unspaced_list_marker.md");
    let diagnostics = lint_file("unspaced_list_marker.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unspaced-list-marker")
        .collect();

    assert_eq!(issues.len(), 2, "{issues:#?}");
    let positions: Vec<_> = issues
        .iter()
        .map(|d| (d.location.line, d.location.column))
        .collect();
    assert_eq!(positions, vec![(2, 3), (8, 4)]);
    for diagnostic in issues {
        assert_eq!(diagnostic.severity, panache::linter::Severity::Warning);
        assert_eq!(&input[diagnostic.location.range], "-\\");
        assert!(diagnostic.fix.is_none());
    }
}

#[test]
fn test_unspaced_list_marker_can_be_disabled() {
    let diagnostics = lint_file_with_config(
        "unspaced_list_marker.md",
        "[lint.rules]\nunspaced-list-marker = false\n",
    );
    assert!(diagnostics.iter().all(|d| d.code != "unspaced-list-marker"));
}

#[test]
fn test_unspaced_list_marker_respects_ignore_directives() {
    let input = format!(
        "<!-- panache-ignore-lint-start -->\n\n{}\n<!-- panache-ignore-lint-end -->\n",
        include_str!("linting/unspaced_list_marker.md")
    );
    let config = Config::default();
    let tree = panache::parse(&input, Some(config.clone()));
    let diagnostics = lint(&tree, &input, &config);
    assert!(diagnostics.iter().all(|d| d.code != "unspaced-list-marker"));
}

#[test]
fn test_unsupported_metadata_key() {
    let diagnostics =
        lint_file_with_config("unsupported_metadata_key.qmd", "flavor = \"quarto\"\n");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "unsupported-metadata-key")
        .collect();

    // `[flow]`, `&anchored [x]`, the nested `? [a, b]`, the `[d, e]` key inside
    // a flow mapping, and the `*scalar` alias key. Scalar keys (`1:`, `no:`)
    // and collections in value position are not flagged.
    assert_eq!(issues.len(), 5, "expected 5 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![5, 6, 8, 10, 12]);
    assert!(
        issues
            .iter()
            .all(|d| d.severity == panache::linter::Severity::Error)
    );
    assert!(issues.iter().all(|d| d.fix.is_none()));
    assert!(
        issues[4].message.contains("alias"),
        "the alias key should say so, got {:?}",
        issues[4].message
    );
    assert!(
        issues[0]
            .notes
            .iter()
            .any(|n| n.message.contains("Non-string keys are not supported")),
        "expected pandoc's error text in a note, got {:#?}",
        issues[0].notes
    );
}

#[test]
fn test_unsupported_metadata_key_not_registered_for_commonmark() {
    let diagnostics =
        lint_file_with_config("unsupported_metadata_key.qmd", "flavor = \"commonmark\"\n");
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "unsupported-metadata-key"),
        "pandoc never reads CommonMark frontmatter as metadata"
    );
}

#[test]
fn test_footnote_swallowed_by_bracket() {
    let diagnostics = lint_file("footnote_swallowed_by_bracket.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "footnote-swallowed-by-bracket")
        .collect();

    // Only the reference-label and inline-destination forms are swallowed;
    // the spaced, escaped, and ordinary-follower lines are clean.
    assert_eq!(issues.len(), 2, "expected 2 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![3, 5]);

    // The span points at the `^[` that failed to open a footnote.
    for issue in &issues {
        assert_eq!(
            u32::from(issue.location.range.len()),
            2,
            "expected a two-byte `^[` span, got {:#?}",
            issue.location
        );
    }

    // Reference-label form: intent is unambiguous, so the fix is safe.
    let bracket_fix = issues[0].fix.as_ref().expect("bracket form should fix");
    assert_eq!(bracket_fix.safety, panache::linter::FixSafety::Safe);
    assert_eq!(bracket_fix.edits.len(), 1);
    assert_eq!(bracket_fix.edits[0].replacement, " ");

    // Inline-destination form could be a stray caret, so the fix is unsafe.
    let paren_fix = issues[1].fix.as_ref().expect("paren form should fix");
    assert_eq!(paren_fix.safety, panache::linter::FixSafety::Unsafe);
}

#[test]
fn test_footnote_swallowed_by_bracket_needs_inline_footnotes() {
    // CommonMark has no `^[note]` syntax, so `^` before a link is just a
    // caret and there is nothing to warn about.
    let diagnostics = lint_file_with_config(
        "footnote_swallowed_by_bracket.md",
        "flavor = \"commonmark\"\n",
    );
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "footnote-swallowed-by-bracket"),
        "rule should be gated off without extensions.inline-footnotes"
    );
}

#[test]
fn test_reversed_footnote_marker() {
    let diagnostics = lint_file("reversed_footnote_marker.md");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "reversed-footnote-marker")
        .collect();

    // The backwards markers: single-line prose, prose across a line break,
    // prose that degrades into a citation, and one whose swap would build a
    // link. The bare label, the inline footnote, and the caret-led link text
    // are clean.
    assert_eq!(issues.len(), 4, "expected 4 diagnostics, got {:#?}", issues);
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![3, 5, 8, 19]);

    // The span points at the `[^` that failed to open a footnote reference.
    for issue in &issues {
        assert_eq!(
            u32::from(issue.location.range.len()),
            2,
            "expected a two-byte `[^` span, got {:#?}",
            issue.location
        );
    }

    for issue in &issues[..3] {
        let fix = issue.fix.as_ref().expect("marker swap fix");
        assert_eq!(fix.safety, panache::linter::FixSafety::Unsafe);
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].replacement, "^[");
    }

    // A `(` right after the closing `]` makes the swap produce a link, not a
    // footnote, so no fix is offered.
    assert!(
        issues[3].fix.is_none(),
        "expected no fix, got {:#?}",
        issues[3].fix
    );

    // The reference-not-found rule stands down on these, so the reader gets
    // one accurate diagnostic instead of two conflicting ones.
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "undefined-footnote-id" || d.code == "undefined-reference-label"),
        "undefined-references should not double-report: {:#?}",
        diagnostics
    );
}

#[test]
fn test_reversed_footnote_marker_needs_inline_footnotes() {
    // GFM footnote labels go through markdown-it, which accepts spaces, so
    // there is no reversed marker to warn about.
    let diagnostics = lint_file_with_config("reversed_footnote_marker.md", "flavor = \"gfm\"\n");
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.code == "reversed-footnote-marker"),
        "rule should be gated off without extensions.inline-footnotes"
    );
}

#[test]
fn test_footnote_after_image() {
    let diagnostics = lint_file_with_config("footnote_after_image.qmd", "flavor = \"quarto\"\n");
    let issues: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "footnote-after-image")
        .collect();

    assert_eq!(issues.len(), 3, "expected 3 diagnostics, got {:#?}", issues);
    // Next-line footnote, same-line footnote, next-line footnote reference.
    let lines: Vec<usize> = issues.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![4, 8, 13]);
    assert!(issues.iter().all(|d| d.fix.is_none()));
    assert!(
        issues[0].message.contains("figure"),
        "message should name the failure mode, got {:?}",
        issues[0].message
    );
    // The `{#fig-1}` id means a Quarto cross-reference breaks too.
    assert!(
        issues[0]
            .notes
            .iter()
            .any(|n| n.message.contains("cross-references")),
        "expected a cross-reference note, got {:#?}",
        issues[0].notes
    );
}

#[test]
fn test_quarto_schema_frontmatter_and_cells() {
    // unknown-key is opt-in; enable it so this exercises both code families.
    let diagnostics = lint_file_with_config(
        "quarto_schema.qmd",
        "flavor = \"quarto\"\n[lint.rules]\nquarto-schema-unknown-key = true\n",
    );
    let schema_diags: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.starts_with("quarto-schema-"))
        .collect();

    // Frontmatter: `forrmat` (typo) + `toc: maybe` (type). Cell: `echo: maybe`
    // (type) + `eccho` (typo). `title` and the custom `my-custom-field` are not
    // flagged.
    assert_eq!(schema_diags.len(), 4, "got: {schema_diags:?}");
    assert!(
        schema_diags
            .iter()
            .any(|d| d.code == "quarto-schema-unknown-key"
                && d.message.contains("forrmat")
                && d.message.contains("format")),
        "expected unknown-key suggestion for forrmat"
    );
    assert!(
        schema_diags
            .iter()
            .any(|d| d.code == "quarto-schema-unknown-key"
                && d.message.contains("eccho")
                && d.message.contains("echo")),
        "expected unknown-key suggestion for the cell option eccho"
    );
    assert_eq!(
        schema_diags
            .iter()
            .filter(|d| d.code == "quarto-schema-type-mismatch")
            .count(),
        2,
        "expected two type mismatches (frontmatter toc, cell echo)"
    );
}

#[test]
fn test_quarto_schema_unknown_key_off_by_default() {
    // With only the flavor set, type/enum checks fire but unknown-key does not:
    // Quarto itself tolerates unknown keys, so the rule is opt-in.
    let diagnostics = lint_file_with_config("quarto_schema.qmd", "flavor = \"quarto\"");
    let schema_diags: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code.starts_with("quarto-schema-"))
        .collect();

    assert!(
        schema_diags
            .iter()
            .all(|d| d.code != "quarto-schema-unknown-key"),
        "unknown-key must be off by default, got: {schema_diags:?}"
    );
    assert_eq!(
        schema_diags
            .iter()
            .filter(|d| d.code == "quarto-schema-type-mismatch")
            .count(),
        2,
        "type mismatches still fire by default: {schema_diags:?}"
    );
}

#[test]
fn test_quarto_schema_does_not_run_for_pandoc() {
    // Default flavor is Pandoc; the rule must not fire.
    let diagnostics = lint_file("quarto_schema.qmd");
    assert!(
        diagnostics
            .iter()
            .all(|d| !d.code.starts_with("quarto-schema-")),
        "quarto-schema must not run under Pandoc"
    );
}

#[test]
fn test_consumer_divergence() {
    let diagnostics = lint_file_with_config(
        "consumer_divergence.qmd",
        r#"
flavor = "quarto"
"#,
    );
    let divergent: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "consumer-divergence")
        .collect();

    // `country: no`, `shipped: yes`, and `mode: 0755` resolve differently under
    // pandoc (1.1) and js-yaml (1.2). `draft: false`, `retries: 3`,
    // `ratio: 3.14`, the quoted `label: "no"`, and the plain string
    // `region: Norway` are all unambiguous.
    assert_eq!(divergent.len(), 3, "got: {divergent:?}");
    assert!(divergent.iter().any(|d| d.message.contains("country")));
    assert!(divergent.iter().any(|d| d.message.contains("shipped")));
    assert!(divergent.iter().any(|d| d.message.contains("mode")));
    // Each carries an unsafe quoting fix.
    assert!(divergent.iter().all(|d| {
        d.fix
            .as_ref()
            .is_some_and(|f| f.safety == panache::linter::FixSafety::Unsafe)
    }));
}

#[test]
fn test_consumer_divergence_does_not_run_for_pandoc() {
    // Default flavor is Pandoc; frontmatter is libyaml-only, so there is no
    // cross-consumer divergence to flag.
    let diagnostics = lint_file("consumer_divergence.qmd");
    assert!(
        diagnostics.iter().all(|d| d.code != "consumer-divergence"),
        "consumer-divergence must not run under Pandoc"
    );
}

#[test]
fn test_empty_values() {
    let diagnostics = lint_file("empty_values.md");
    let empty: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "empty-values")
        .collect();

    // `title:` and `tags:` are implicit nulls; `author` has a value and
    // `date: null` is an explicit null, so neither is flagged.
    assert_eq!(empty.len(), 2, "got: {empty:?}");
    assert!(empty.iter().any(|d| d.message.contains("title")));
    assert!(empty.iter().any(|d| d.message.contains("tags")));
    // Each flagged key offers an unsafe removal fix.
    assert!(empty.iter().all(|d| {
        d.fix
            .as_ref()
            .is_some_and(|f| f.safety == panache::linter::FixSafety::Unsafe)
    }));
}

#[test]
fn test_citation_nonbreaking_space() {
    let diagnostics = lint_file("citation_nonbreaking_space.md");
    let flagged: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "citation-nonbreaking-space")
        .collect();

    // The plain-space citation on line 1 and the softbreak citation on
    // lines 3-4; the tied, in-text, and paragraph-initial ones are clean.
    assert_eq!(flagged.len(), 2, "got: {flagged:?}");
    assert_eq!(flagged[0].location.line, 1);
    assert_eq!(flagged[1].location.line, 3);
    assert!(flagged.iter().all(|d| {
        d.fix
            .as_ref()
            .is_some_and(|f| f.edits.len() == 1 && f.edits[0].replacement == "\\ ")
    }));
}

#[test]
fn test_table_column_count() {
    let diagnostics = lint_file("table_column_count.md");
    let flagged: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == "table-column-count")
        .collect();

    // Only the middle table overflows: its header and its one body row each
    // carry a third cell the two-column delimiter row drops. The matched
    // table and the short-row table are clean.
    assert_eq!(flagged.len(), 2, "got: {flagged:?}");
    let lines: Vec<usize> = flagged.iter().map(|d| d.location.line).collect();
    assert_eq!(lines, vec![12, 14]);
    assert!(
        flagged.iter().all(|d| d.fix.is_none()),
        "the repair is ambiguous, so no fix ships"
    );
    assert!(
        flagged[0].message.contains("declares 2 columns"),
        "message names the delimiter's count: {}",
        flagged[0].message
    );
}
