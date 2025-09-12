use quartofmt::format;
use quartofmt::{ConfigBuilder, format_tree, parse};

#[test]
fn math_no_wrap() {
    let cfg = ConfigBuilder::default().line_width(10).build();
    let input = "$$\n\\begin{matrix}\n  A & B\\\\\n  C & D\n\\end{matrix}\n$$\n";
    let output = format(input, Some(cfg));

    // Math blocks should not be wrapped
    similar_asserts::assert_eq!(output, input);
}

#[test]
fn math_with_indent() {
    let input = "$$\nA = B\n$$\n";
    let config = ConfigBuilder::default().math_indent(2).build();
    let tree = parse(input);
    let output = format_tree(&tree, &config);
    // Assert output is indented as expected
    assert!(output.contains("  A = B"));
}
