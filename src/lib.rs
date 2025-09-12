pub mod config;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod syntax;

pub use formatter::format_tree;
pub use parser::parse;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

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
    #[cfg(debug_assertions)]
    {
        init_logger();
    }
    // Normalize line endings to Unix style first
    let normalized_input = input.replace("\r\n", "\n");
    let tree = parse(&normalized_input);
    format_tree(&tree, line_width.unwrap_or(80))
}
