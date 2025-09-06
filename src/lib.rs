pub mod config;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod syntax;

pub use formatter::format_tree;
pub use parser::parse;

/// Formats a Quarto document string with the specified line width.
///
/// This function normalizes line endings, preserves code blocks and frontmatter,
/// and applies consistent paragraph wrapping.
///
/// # Examples
///
/// ```rust
/// use quartofmt::format;
///
/// let input = "This is a very long line that should be wrapped.";
/// let formatted = format(input, Some(80));
/// ```
///
/// # Arguments
///
/// * `input` - The Quarto document content to format
/// * `line_width` - Optional line width (defaults to 80)
pub fn format(input: &str, line_width: Option<usize>) -> String {
    // Normalize line endings to Unix style first
    let normalized_input = input.replace("\r\n", "\n");
    let tree = parse(&normalized_input);
    format_tree(&tree, line_width.unwrap_or(80))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_and_paragraph() {
        let input = "---\ntitle: hi\n---\n\nHello world\n";
        let output = format(input, Some(80));

        // Basic roundtrip test - the exact formatting might change
        assert!(output.contains("title: hi"));
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn code_cell_roundtrip() {
        let input = "```{r}\nprint(1)\n```\n";
        let output = format(input, Some(80));

        // Code blocks should be preserved exactly
        assert_eq!(output, input);
    }

    #[test]
    fn paragraph_wrapping() {
        let input =
            "This is a long line that should be wrapped to a shorter width for testing purposes.\n";
        let output = format(input, Some(20));

        // Check that lines are wrapped
        for line in output.lines() {
            assert!(line.len() <= 20, "Line too long: '{line}'");
        }
    }

    #[test]
    fn blank_line_preservation() {
        let input = "First paragraph\n\n\nSecond paragraph\n";
        let output = format(input, Some(80));

        // Should preserve the double blank line between paragraphs
        let expected = "First paragraph\n\n\nSecond paragraph\n";
        assert_eq!(output, expected);

        // Also test that we don't add extra blank lines
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines[1], ""); // First blank line
        assert_eq!(lines[2], ""); // Second blank line
    }

    #[test]
    fn paragraph_wrapping_edge_cases() {
        // Test 1: Paragraph with internal line breaks should be reflowed
        let input = "This is a long\nsentence that should\nbe wrapped properly.\n";
        let output = format(input, Some(25));

        // Should reflow the text, not preserve internal line breaks
        assert!(!output.contains("long\nsentence"));
        assert!(output.lines().all(|line| line.len() <= 25));

        // Test 2: Multiple spaces should be normalized
        let input2 = "Word1    word2     word3\n";
        let output2 = format(input2, Some(80));
        assert_eq!(output2, "Word1 word2 word3\n");

        // Test 3: Leading/trailing whitespace should be trimmed
        let input3 = "  Leading and trailing  \n";
        let output3 = format(input3, Some(80));
        assert_eq!(output3, "Leading and trailing\n");
    }

    #[test]
    fn math_no_wrap() {
        let input = "$$\n\\begin{matrix}\n  A & B\\\\\n  C & D\n\\end{matrix}\n$$\n";
        let output = format(input, Some(10));

        // Math blocks should not be wrapped
        assert_eq!(output, input);
    }

    #[test]
    fn fenced_div_roundtrip() {
        let input = "::: {.my-div}\nSome div content\n:::\n";
        let output = format(input, Some(80));

        // Fenced divs should be preserved exactly
        assert_eq!(output, input);
    }

    #[test]
    fn quote_single_line() {
        let input = "> This is a single line quote.\n";
        let output = format(input, Some(80));

        assert!(output.starts_with("> "));
        assert!(output.contains("single line quote"));
    }

    #[test]
    fn quote_multi_line_continuous() {
        let input = "> This is a multi-line quote\n> that continues on the next line.\n";
        let output = format(input, Some(80));

        for line in output.lines() {
            assert!(
                line.starts_with("> "),
                "Line should start with '>': '{line}'"
            );
        }
        assert!(output.contains("multi-line quote"));
        assert!(output.contains("continues on the next line"));
    }

    #[test]
    fn quote_with_wrapping() {
        let input = "> This is a very long quote that should definitely be wrapped when the line width is set to a small value like twenty characters.\n";
        let output = format(input, Some(25));

        for line in output.lines() {
            if !line.is_empty() {
                assert!(
                    line.starts_with("> "),
                    "Line should start with '>': '{line}'"
                );
                assert!(line.len() <= 25, "Line too long: '{line}'");
            }
        }
    }

    #[test]
    fn quote_with_blank_lines() {
        let input = "> First paragraph in quote\n>\n> Second paragraph in quote\n";
        let output = format(input, Some(80));

        for line in output.lines() {
            // All lines should start with ">", but blank quote lines are just ">"
            assert!(
                line.starts_with(">"),
                "Line should start with '>': '{line}'"
            );
        }
        assert!(output.contains("First paragraph"));
        assert!(output.contains("Second paragraph"));
        // Should have a blank quote line
        assert!(output.contains(">\n"));
    }

    #[test]
    fn markdown_link_no_break() {
        let input = "after this line comes a link ![a link](https://alink.com)\n";
        let output = format(input, Some(30));

        // The ![a link](https://alink.com) should stay together
        assert!(
            !output.contains("!\n["),
            "Image link should not be broken across lines"
        );

        assert!(
            !output.contains("]\n("),
            "Link text and URL should not be separated"
        );

        // Test regular links too - they can be broken, but not at critical points
        let input2 = "here is a regular [link text](https://example.com) in text\n";
        let output2 = format(input2, Some(25));

        // Regular links can be broken, but shouldn't break ](
        assert!(
            !output2.contains("]\n("),
            "Link text and URL should not be separated"
        );

        // The link should still be functional
        assert!(output2.contains("https://example.com"));
    }

    #[test]
    fn list_roundtrip() {
        let input = "- First item\n- Second item\n";
        let output = format(input, Some(80));
        assert_eq!(output, input);
    }

    #[test]
    fn list_wrapping() {
        let input = "- A list with items that should wrap properly and retain their markers\n- Second item with more text to wrap\n";
        let output = format(input, Some(31));
        let expected = "- A list with items that should\n  wrap properly and retain\n  their markers\n- Second item with more text\n  to wrap\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn nested_list_wrapping() {
        let input = "- Top level\n  - Nested level 1 with some text that should wrap\n    - Nested level 2 with even more text to wrap and demonstrate nesting\n";
        let output = format(input, Some(32));
        let expected = "- Top level\n  - Nested level 1 with some\n    text that should wrap\n    - Nested level 2 with even\n      more text to wrap and\n      demonstrate nesting\n";
        assert_eq!(output, expected);
    }

    #[test]
    fn list_item_link_no_break() {
        let input = "- A list item with a link ![some link that is very long](./example.com/very/long/path/to/file) in it\n";
        let output = format(input, Some(30));

        // The link should not be broken at the ]( boundary
        assert!(
            !output.contains("]\n("),
            "Link text and URL should not be separated in list items"
        );

        // The link should still be functional
        assert!(output.contains("./example.com/very/long/path/to/file"));
    }

    #[test]
    fn nested_divs_roundtrip() {
        let input = ":::: columns\n\n::: column\n\nColumn 1 content\n\n:::\n\n::: column\n\nColumns 2 content\n\n:::\n\n::::\n";
        let output = format(input, Some(80));
        assert_eq!(output, input);
    }

    #[test]
    fn div_paragraph_wrapping() {
        let input = "::: {.my-div}\nThis is a very long paragraph inside a fenced div that should be wrapped to a shorter width for testing purposes.\n:::\n";
        let output = format(input, Some(30));
        // Check that the output starts with the opening fence and contains wrapped lines
        assert!(output.starts_with("::: {.my-div}\n"));
        for line in output.lines().skip(1).take(4) {
            assert!(line.len() <= 30, "Line too long: '{line}'");
        }
        assert!(output.ends_with(":::\n"));
    }
}
