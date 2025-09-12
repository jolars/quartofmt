use quartofmt::format;

#[test]
fn fenced_div_roundtrip() {
    let input = "::: {.my-div}\nSome div content\n:::\n";
    let output = format(input, Some(80));

    // Fenced divs should be preserved exactly
    assert_eq!(output, input);
}
