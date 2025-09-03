pub mod config;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod syntax;

pub use formatter::format_tree;
pub use parser::parse;

/// Main formatting function
pub fn format_str(input: &str, line_width: Option<usize>) -> String {
    let tree = parse(input);
    format_tree(&tree, line_width.unwrap_or(80))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_and_paragraph() {
        let input = "---\ntitle: hi\n---\n\nHello world\n";
        let output = format_str(input, Some(80));

        // Basic roundtrip test - the exact formatting might change
        assert!(output.contains("title: hi"));
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn code_cell_roundtrip() {
        let input = "```{r}\nprint(1)\n```\n";
        let output = format_str(input, Some(80));

        // Code blocks should be preserved exactly
        assert_eq!(output, input);
    }

    #[test]
    fn paragraph_wrapping() {
        let input =
            "This is a long line that should be wrapped to a shorter width for testing purposes.\n";
        let output = format_str(input, Some(20));

        // Check that lines are wrapped
        for line in output.lines() {
            assert!(line.len() <= 20, "Line too long: '{line}'");
        }
    }

    #[test]
    fn blank_line_preservation() {
        let input = "First paragraph\n\n\nSecond paragraph\n";
        let output = format_str(input, Some(80));

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
        let output = format_str(input, Some(25));

        // Should reflow the text, not preserve internal line breaks
        assert!(!output.contains("long\nsentence"));
        assert!(output.lines().all(|line| line.len() <= 25));

        // Test 2: Multiple spaces should be normalized
        let input2 = "Word1    word2     word3\n";
        let output2 = format_str(input2, Some(80));
        assert_eq!(output2, "Word1 word2 word3\n");

        // Test 3: Leading/trailing whitespace should be trimmed
        let input3 = "  Leading and trailing  \n";
        let output3 = format_str(input3, Some(80));
        assert_eq!(output3, "Leading and trailing\n");
    }
}
