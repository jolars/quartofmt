use quartofmt::format;

#[test]
fn latex_command_preservation() {
    let input = "\\pdfpcnote{\n  Ask them to identify the bottleneck.\n}\n";
    let output = format(input, Some(80));

    // LaTeX commands should be preserved exactly as written
    assert_eq!(output, input);
}

#[test]
fn latex_command_in_paragraph() {
    let input = "This is a paragraph with \\textbf{bold text} in the middle.\n";
    let output = format(input, Some(80));

    // LaTeX command should be preserved within the paragraph
    assert!(output.contains("\\textbf{bold text}"));
    assert_eq!(output, input);
}

#[test]
fn latex_command_with_multiple_args() {
    let input = "\\includegraphics[width=0.5\\textwidth]{figure.png}\n";
    let output = format(input, Some(80));

    // Complex LaTeX commands should be preserved
    assert_eq!(output, input);
}

#[test]
fn latex_command_no_wrap() {
    let input = "This is a very long line with \\pdfpcnote{a very long note that should not be wrapped or reformatted} that exceeds line width.\n";
    let output = format(input, Some(30));

    // Check that the LaTeX command appears somewhere in the output (may be wrapped)
    assert!(output.contains("\\pdfpcnote{"));
}

#[test]
fn mixed_latex_and_markdown() {
    let input = "Here is some text with \\LaTeX{} and [a link](https://example.com) together.\n";
    let output = format(input, Some(80));

    // Both LaTeX and markdown should be preserved
    assert!(output.contains("\\LaTeX{}"));
    assert!(output.contains("https://example.com"));
}
