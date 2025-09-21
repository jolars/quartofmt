use quartofmt::format;

#[test]
fn wrapping() {
    let cfg = quartofmt::ConfigBuilder::default().line_width(20).build();
    let input =
        "This is a long line that should be wrapped to a shorter width for testing purposes.\n";
    let output = format(input, Some(cfg));

    // Check that lines are wrapped
    for line in output.lines() {
        assert!(line.len() <= 20, "Line too long: '{line}'");
    }
}

#[test]
fn paragraph_wrapping_edge_cases() {
    // Test 1: Paragraph with internal line breaks should be reflowed
    let cfg = quartofmt::ConfigBuilder::default().line_width(25).build();
    let input = "This is a long\nsentence that should\nbe wrapped properly.\n";
    let output = format(input, Some(cfg));

    // Should reflow the text, not preserve internal line breaks
    assert!(!output.contains("long\nsentence"));
    assert!(output.lines().all(|line| line.len() <= 25));

    // Test 2: Multiple spaces should be normalized
    let input2 = "Word1    word2     word3\n";
    let output2 = format(input2, None);
    similar_asserts::assert_eq!(output2, "Word1 word2 word3\n");

    // Test 3: Leading/trailing whitespace should be trimmed
    let input3 = "  Leading and trailing  \n";
    let output3 = format(input3, None);
    similar_asserts::assert_eq!(output3, "Leading and trailing\n");
}

#[test]
fn preserves_inline_code_whitespace() {
    let input = "This is `foo   bar` inline code.";
    let output = format(input, None);
    similar_asserts::assert_eq!(output, "This is `foo   bar` inline code.\n");
}

#[test]
fn preserves_inline_math_whitespace() {
    let input = "Math: $x   +   y$";
    let output = format(input, None);
    similar_asserts::assert_eq!(output, "Math: $x   +   y$\n");
}
