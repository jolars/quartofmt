use quartofmt::format;

#[test]
fn quote_single_line() {
    let input = "> This is a single line quote.\n";
    let output = format(input, Some(80));

    assert!(output.starts_with("> "));
    assert!(output.contains("single line quote"));
}

#[test]
fn quote_multi_line_continuous() {
    let input = "> This is a multi-line quote\n> that continues on the next line.\n";
    let output = format(input, Some(80));

    for line in output.lines() {
        assert!(
            line.starts_with("> "),
            "Line should start with '>': '{line}'"
        );
    }
    assert!(output.contains("multi-line quote"));
    assert!(output.contains("continues on the next line"));
}

#[test]
fn quote_with_wrapping() {
    let input = "> This is a very long quote that should definitely be wrapped when the line width is set to a small value like twenty characters.\n";
    let output = format(input, Some(25));

    for line in output.lines() {
        if !line.is_empty() {
            assert!(
                line.starts_with("> "),
                "Line should start with '>': '{line}'"
            );
            assert!(line.len() <= 25, "Line too long: '{line}'");
        }
    }
}

#[test]
fn quote_with_blank_lines() {
    let input = "> First paragraph in quote\n>\n> Second paragraph in quote\n";
    let output = format(input, Some(80));

    for line in output.lines() {
        // All lines should start with ">", but blank quote lines are just ">"
        assert!(
            line.starts_with(">"),
            "Line should start with '>': '{line}'"
        );
    }
    assert!(output.contains("First paragraph"));
    assert!(output.contains("Second paragraph"));
    // Should have a blank quote line
    assert!(output.contains(">\n"));
}
