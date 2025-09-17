use quartofmt::format;

#[test]
fn atx_trailing_hashes_are_removed() {
    let input = "### A level-three heading ###\n";
    let expected = "### A level-three heading\n";
    let out = format(input, None);
    assert_eq!(out, expected);

    // idempotent
    assert_eq!(format(&out, None), expected);
}

#[test]
fn atx_leading_spaces_are_normalized() {
    let input = "   ##   Title   \n";
    let expected = "## Title\n";
    let out = format(input, None);
    assert_eq!(out, expected);
    assert_eq!(format(&out, None), expected);
}

#[test]
fn setext_level_one_to_atx() {
    let input = "A level-one heading\n====================\n";
    let expected = "# A level-one heading\n";
    let out = format(input, None);
    assert_eq!(out, expected);
    assert_eq!(format(&out, None), expected);
}

#[test]
fn setext_level_two_to_atx() {
    let input = "A level-two heading\n-------------------\n";
    let expected = "## A level-two heading\n";
    let out = format(input, None);
    assert_eq!(out, expected);
    assert_eq!(format(&out, None), expected);
}

#[test]
fn heading_with_inline_formatting_preserved() {
    let input = "A heading with a [link](/url) and *emphasis*\n================================================\n";
    let expected = "# A heading with a [link](/url) and *emphasis*\n";
    let out = format(input, None);
    assert_eq!(out, expected);
    assert_eq!(format(&out, None), expected);
}
