use quartofmt::format;

#[test]
fn list_roundtrip() {
    let input = "- First item\n- Second item\n";
    let output = format(input, Some(80));
    similar_asserts::assert_eq!(output, input);
}

#[test]
fn list_wrapping() {
    let input = "- A list with items that should wrap properly and retain their markers\n- Second item with more text to wrap\n";
    let output = format(input, Some(31));
    let expected = "- A list with items that should\n  wrap properly and retain\n  their markers\n- Second item with more text\n  to wrap\n";
    similar_asserts::assert_eq!(output, expected);
}

#[test]
fn nested_list_wrapping() {
    let input = "- Top level\n  - Nested level 1 with some text that should wrap\n    - Nested level 2 with even more text to wrap and demonstrate nesting\n";
    let output = format(input, Some(32));
    let expected = "- Top level\n  - Nested level 1 with some\n    text that should wrap\n    - Nested level 2 with even\n      more text to wrap and\n      demonstrate nesting\n";
    similar_asserts::assert_eq!(output, expected);
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
    similar_asserts::assert_eq!(output, input);
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

#[test]
fn list_items_separate_properly() {
    let input = "- In R, objects are passed by reference, but when an object is modified a copy\n  is created.\n- For instance, when subsetting a matrix, a copy is created. It's not possible\n  to access for instance a column by reference.\n";
    let output = format(input, Some(80));

    // Should have two distinct list items
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines[0].starts_with("- In R"));
    assert!(lines[1].starts_with("  is created"));
    assert!(lines[2].starts_with("- For instance"));

    // Should not merge the list items
    assert!(!output.contains("is created. - For instance"));
}
