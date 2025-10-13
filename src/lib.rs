pub mod block_parser;
pub mod config;
pub mod formatter;
pub mod inline_parser;
pub mod syntax;

pub use config::Config;
pub use config::ConfigBuilder;
pub use formatter::format_tree;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

fn detect_line_ending(input: &str) -> &str {
    // Check for first occurrence of \r\n or \n
    let rn_pos = input.find("\r\n");
    let n_pos = input.find('\n');

    if let (Some(rn), Some(n)) = (rn_pos, n_pos) {
        if rn < n {
            return "\r\n";
        }
    } else if rn_pos.is_some() {
        return "\r\n";
    }

    "\n"
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
/// let cfg = quartofmt::ConfigBuilder::default().line_width(80).build();
///
/// let input = "This is a very long line that should be wrapped.";
/// let formatted = format(input, Some(cfg));
/// ```
///
/// # Arguments
///
/// * `input` - The Quarto document content to format
/// * `line_width` - Optional line width (defaults to 80)
pub fn format(input: &str, config: Option<Config>) -> String {
    #[cfg(debug_assertions)]
    {
        init_logger();
    }

    let line_ending = detect_line_ending(input);

    let normalized_input = input.replace("\r\n", "\n");

    // Step 1: Parse blocks to create initial CST
    let block_tree = block_parser::BlockParser::new(&normalized_input).parse();

    // Step 2: Run inline parser on block content to create final CST
    let tree = inline_parser::InlineParser::new(block_tree).parse();

    // Step 3: Format the final CST
    let config = config.unwrap_or_default();
    let out = format_tree(&tree, &config);

    if line_ending == "\r\n" {
        out.replace("\n", "\r\n")
    } else {
        out
    }
}

pub fn format_with_defaults(input: &str) -> String {
    format(input, None)
}
