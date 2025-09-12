use quartofmt::format;

#[test]
fn front_matter_and_paragraph() {
    let input = "---\ntitle: hi\n---\n\nHello world\n";
    let output = format(input, Some(80));

    // Basic roundtrip test - the exact formatting might change
    assert!(output.contains("title: hi"));
    assert!(output.contains("Hello world"));
}
