use quartofmt::ConfigBuilder;
use quartofmt::format;

#[test]
fn math_no_wrap() {
    let cfg = ConfigBuilder::default().line_width(10).build();
    let input = "$$\n\\begin{matrix}\nA & B\\\\\nC & D\n\\end{matrix}\n$$\n";
    let output = format(input, Some(cfg));

    // Math blocks should not be wrapped
    similar_asserts::assert_eq!(output, input);
}
