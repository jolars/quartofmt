use quartofmt::format;

#[test]
fn simple_table_roundtrip() {
    let input = "\
        Header 1  Header 2
        --------  --------
        Cell 1    Cell 2
        Cell 3    Cell 4
    ";
    let output = format(input, None);
    similar_asserts::assert_eq!(output, input);
}
