use quartofmt::format;

#[test]
fn markdown_link_no_break() {
    let input = "after this line comes a link ![a link](https://alink.com)\n";
    let output = format(input, Some(30));

    // The ![a link](https://alink.com) should stay together
    assert!(
        !output.contains("!\n["),
        "Image link should not be broken across lines"
    );

    assert!(
        !output.contains("]\n("),
        "Link text and URL should not be separated"
    );

    // Test regular links too - they can be broken, but not at critical points
    let input2 = "here is a regular [link text](https://example.com) in text\n";
    let output2 = format(input2, Some(25));

    // Regular links can be broken, but shouldn't break ](
    assert!(
        !output2.contains("]\n("),
        "Link text and URL should not be separated"
    );

    // The link should still be functional
    assert!(output2.contains("https://example.com"));
}
