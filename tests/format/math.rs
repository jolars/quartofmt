use quartofmt::format;

#[test]
fn math_no_wrap() {
    let input = "$$\n\\begin{matrix}\n  A & B\\\\\n  C & D\n\\end{matrix}\n$$\n";
    let output = format(input, Some(10));

    // Math blocks should not be wrapped
    similar_asserts::assert_eq!(output, input);
}
