use quartofmt::format;

#[test]
fn numbered_list_roundtrip() {
    let input = "1. First item\n2. Second item\n3. Third item\n";
    let output = format(input, Some(80));
    assert_eq!(output, input);
}

#[test]
fn numbered_list_formatting() {
    let input = "1. First item\n2. Second item\n3. Third item\n";
    let output = format(input, Some(80));

    // Should preserve numbered list structure
    assert_eq!(output, input);

    // Test with wrapping
    let input_long = "1. This is a very long numbered list item that should wrap properly when the line width is constrained\n2. Second item with more text to wrap around\n";
    let output_long = format(input_long, Some(30));

    let lines: Vec<&str> = output_long.lines().collect();
    assert!(lines[0].starts_with("1. "));
    assert!(lines[1].starts_with("   ")); // Continuation should be indented
    assert!(lines.iter().any(|line| line.starts_with("2. ")));

    // Should not merge numbered items
    assert!(!output_long.contains("wrap properly when the line width is constrained 2. Second"));
}
