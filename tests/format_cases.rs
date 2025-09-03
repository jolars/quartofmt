use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use quartofmt::format_str;

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("cases")
}

fn run_case(dir: &Path) -> io::Result<()> {
    let input_path = dir.join("input.qmd");
    let expected_path = dir.join("expected.qmd");

    let input = fs::read_to_string(&input_path)?;
    let expected = match fs::read_to_string(&expected_path) {
        Ok(s) => s,
        Err(_) => input.clone(), // default to round-trip if expected is absent
    };

    let output = format_str(&input, Some(80));

    if output != expected {
        let diff = diff::lines(&expected, &output)
            .into_iter()
            .map(|d| match d {
                diff::Result::Left(l) => format!("-{l}"),
                diff::Result::Right(r) => format!("+{r}"),
                diff::Result::Both(b, _) => format!(" {b}"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "Mismatch in case: {}\nDiff:\n{}",
            dir.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default(),
            diff
        );
    }

    Ok(())
}

#[test]
fn golden_cases() -> io::Result<()> {
    let root = cases_root();
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            run_case(&path)?;
        }
    }
    Ok(())
}
