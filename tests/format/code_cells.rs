use quartofmt::format;

#[test]
fn code_cell_roundtrip() {
    let input = "```{r}\nprint(1)\n```\n";
    let output = format(input, None);

    // Code blocks should be preserved exactly
    similar_asserts::assert_eq!(output, input);
}
