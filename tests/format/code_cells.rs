use quartofmt::format;

#[test]
fn code_cell_roundtrip() {
    let input = "```{r}\nprint(1)\n```\n";
    let output = format(input, Some(80));

    // Code blocks should be preserved exactly
    assert_eq!(output, input);
}
