use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use clap::Parser;

use quartofmt::format_str;

#[derive(Parser)]
#[command(name = "quartofmt")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A formatter for Quarto documents")]
struct Cli {
    /// Input file to format (stdin if not provided)
    file: Option<PathBuf>,

    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Check if files are formatted without making changes
    #[arg(long)]
    check: bool,

    /// Format files in place
    #[arg(long)]
    write: bool,
}

fn read_all(path: Option<&PathBuf>) -> io::Result<String> {
    match path {
        Some(p) => fs::read_to_string(p),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn start_dir_for(input_path: &Option<PathBuf>) -> io::Result<PathBuf> {
    if let Some(p) = input_path {
        Ok(p.parent().unwrap_or(Path::new(".")).to_path_buf())
    } else {
        std::env::current_dir()
    }
}

fn main() -> io::Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    let start_dir = start_dir_for(&cli.file)?;
    let (cfg, _cfg_path) = quartofmt::config::load(cli.config.as_deref(), &start_dir)?;

    let input = read_all(cli.file.as_ref())?;
    let output = format_str(&input, cfg.line_width);

    if cli.check {
        if input != output {
            eprintln!("File is not formatted");
            std::process::exit(1);
        }
        println!("File is correctly formatted");
    } else if cli.write {
        if let Some(file_path) = &cli.file {
            fs::write(file_path, &output)?;
            println!("Formatted {}", file_path.display());
        } else {
            eprintln!("Cannot use --write with stdin input");
            std::process::exit(1);
        }
    } else {
        print!("{output}");
    }

    Ok(())
}
