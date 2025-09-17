use quartofmt::config::WrapMode;
use quartofmt::{format, Config};

fn cfg_preserve() -> Config {
    Config {
        wrap: Some(WrapMode::Preserve),
        ..Default::default()
    }
}

#[test]
fn paragraph_preserve_keeps_line_breaks() {
    let input = "\
First line with manual
breaks that should
stay the same.
";

    let out = format(input, Some(cfg_preserve()));
    // Idempotency
    let out2 = format(&out, Some(cfg_preserve()));
    assert_eq!(out, out2);

    // Preserve mode should keep paragraph line breaks exactly
    assert_eq!(out, input);
}

#[test]
fn block_quote_preserve_keeps_line_breaks() {
    let input = "\
> First line with manual
> breaks that should
> stay the same.
";

    let out = format(input, Some(cfg_preserve()));
    // Idempotency
    let out2 = format(&out, Some(cfg_preserve()));
    assert_eq!(out, out2);

    // Preserve mode should keep quoted line breaks exactly
    assert_eq!(out, input);
}
