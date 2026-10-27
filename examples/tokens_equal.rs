//! Proves spacify preserved meaning: every `.rs` file under <original> has the same token stream as
//! its copy under <formatted>. A token stream leaves out whitespace and plain comments but keeps
//! literals and doc comments, which is everything a blank line could change. A file that does not
//! tokenize must come back byte for byte, since spacify refuses it.
//!
//! Usage: cargo run --release --example `tokens_equal` -- <original dir> <formatted dir>

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use proc_macro2::TokenStream;

fn main() -> ExitCode {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    let [original, formatted] = args.as_slice() else {
        eprintln!("usage: tokens_equal <original dir> <formatted dir>");
        return ExitCode::from(2);
    };

    let mut files = Vec::new();
    rust_files(original, &mut files);
    files.sort();

    let mut failures = 0;
    let mut checked = 0;
    for path in &files {
        let relative = path.strip_prefix(original).expect("walked from the original directory");
        let before = fs::read(path).expect("readable original");
        let after = fs::read(formatted.join(relative)).expect("readable formatted copy");
        match (tokens(&before), tokens(&after)) {
            (Some(before), Some(after)) if before == after => checked += 1,
            (None, _) if before == after => {}
            (None, _) => {
                eprintln!("spacify rewrote a file that does not tokenize: {}", relative.display());
                failures += 1;
            }
            _ => {
                eprintln!("spacify changed the tokens of {}", relative.display());
                failures += 1;
            }
        }
    }

    if failures > 0 {
        return ExitCode::FAILURE;
    }
    println!("spacify kept the tokens of {checked} files");
    ExitCode::SUCCESS
}

fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable directory") {
        let path = entry.expect("readable entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn tokens(source: &[u8]) -> Option<String> {
    let source = std::str::from_utf8(source).ok()?;
    let stream: TokenStream = source.parse().ok()?;
    Some(stream.to_string())
}
