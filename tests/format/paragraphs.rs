use quartofmt::format;

#[test]
fn wrapping() {
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
