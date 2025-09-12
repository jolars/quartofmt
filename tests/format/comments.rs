use quartofmt::format;

#[test]
fn comment_roundtrip() {
    let input = "<!-- This is a comment -->\n";
    let output = format(input, Some(80));
    assert_eq!(output, input);
}

#[test]
fn comment_within_content() {
    let input =
        "Some text before the comment.\n<!-- This is a comment -->\nSome text after the comment.\n";
    let output = format(input, Some(160));
    assert!(output.contains("Some text before the comment."));
    assert!(output.contains("<!-- This is a comment -->"));
    assert!(output.contains("Some text after the comment."));
}

#[test]
fn comment_no_wrap() {
    let input = "Some text before the comment.\n<!-- This is a very long comment that should not be wrapped or reformatted -->\nSome text after the comment.\n";
    let output = format(input, Some(40));
    assert!(output.contains(
        "<!-- This is a very long comment that should not be wrapped or reformatted -->"
    ));
}
