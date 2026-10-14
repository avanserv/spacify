mod walk;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use similar::TextDiff;

use spacify::config::{Config, Rules};
use spacify::format_source;

/// Formats the vertical rhythm of Python code: blank lines around logical blocks.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Files or directories to format, or `-` for stdin.
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// Write nothing; exit 1 when a file would change.
    #[arg(long)]
    check: bool,

    /// Write nothing; print a diff of what would change and exit 1 when there is any.
    #[arg(long)]
    diff: bool,

    /// Use this configuration file instead of searching upwards for one.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Skip excluded files even when they are named outright, as a git hook does.
    #[arg(long)]
    force_exclude: bool,

    /// The path of the file read from stdin, for messages, exclusion and stub detection.
    #[arg(long, value_name = "PATH")]
    stdin_filename: Option<PathBuf>,

    /// Print errors only.
    #[arg(short, long)]
    quiet: bool,
}

enum Outcome {
    Unchanged,
    Changed,
    Failed,
}

fn main() -> ExitCode {
    match run(&Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: &Cli) -> Result<ExitCode> {
    let cwd = env::current_dir().context("cannot read the current directory")?;
    let config = match &cli.config {
        Some(path) => Config::load(path)?,
        None => Config::discover(&cwd)?,
    };
    let exclude = walk::Exclude::new(&config, &cwd)?;

    if cli.paths.len() == 1 && cli.paths[0].as_os_str() == "-" {
        return format_stdin(cli, &config.rules, &exclude, &cwd);
    }

    let files = walk::collect(&cli.paths, &exclude, &cwd, cli.force_exclude);
    let outcomes: Vec<Outcome> = files
        .par_iter()
        .map(|path| format_file(cli, &config.rules, path))
        .collect();

    let changed = outcomes.iter().filter(|o| matches!(o, Outcome::Changed)).count();
    let failed = outcomes.iter().filter(|o| matches!(o, Outcome::Failed)).count();
    let unchanged = outcomes.len() - changed - failed;
    if !cli.quiet {
        eprintln!("{}", summary(changed, unchanged, cli.check || cli.diff));
    }

    Ok(if failed > 0 {
        ExitCode::from(2)
    } else if changed > 0 && (cli.check || cli.diff) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn format_file(cli: &Cli, rules: &Rules, path: &Path) -> Outcome {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", path.display());
            return Outcome::Failed;
        }
    };

    let formatted = match format_source(&source, rules, is_stub(path)) {
        Ok(Some(formatted)) => formatted,
        Ok(None) => return Outcome::Unchanged,
        Err(error) => {
            eprintln!("error: cannot format {}: {error}", path.display());
            return Outcome::Failed;
        }
    };

    if cli.diff {
        print_diff(&source, &formatted, path);
    } else if cli.check {
        if !cli.quiet {
            report(&format!("Would reformat: {}\n", path.display()));
        }
    } else if let Err(error) = fs::write(path, formatted) {
        eprintln!("error: cannot write {}: {error}", path.display());
        return Outcome::Failed;
    }

    Outcome::Changed
}

fn format_stdin(cli: &Cli, rules: &Rules, exclude: &walk::Exclude, cwd: &Path) -> Result<ExitCode> {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source).context("cannot read stdin")?;
    let name = cli.stdin_filename.as_deref().unwrap_or(Path::new("-"));

    let skip = cli.force_exclude && cli.stdin_filename.is_some() && exclude.matches(name, cwd);
    let formatted = if skip {
        None
    } else {
        format_source(&source, rules, is_stub(name)).with_context(|| format!("cannot format {}", name.display()))?
    };

    let Some(formatted) = formatted else {
        if !(cli.check || cli.diff) {
            io::stdout().write_all(source.as_bytes())?;
        }
        return Ok(ExitCode::SUCCESS);
    };

    if cli.diff {
        print_diff(&source, &formatted, name);
    } else if cli.check {
        if !cli.quiet {
            report(&format!("Would reformat: {}\n", name.display()));
        }
    } else {
        io::stdout().write_all(formatted.as_bytes())?;
        return Ok(ExitCode::SUCCESS);
    }
    Ok(ExitCode::FAILURE)
}

fn is_stub(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "pyi")
}

fn print_diff(source: &str, formatted: &str, path: &Path) {
    let name = path.display().to_string();
    let diff = TextDiff::from_lines(source, formatted);
    report(&diff.unified_diff().header(&name, &name).to_string());
}

/// One write per file, so output from parallel workers never interleaves. A closed pipe, as when
/// the output goes to `head`, is not an error worth dying for.
fn report(text: &str) {
    let _ = io::stdout().lock().write_all(text.as_bytes());
}

fn summary(changed: usize, unchanged: usize, dry_run: bool) -> String {
    let files = |n: usize| {
        if n == 1 {
            "1 file".to_string()
        } else {
            format!("{n} files")
        }
    };
    match (changed, dry_run) {
        (0, _) => format!("{} left unchanged", files(unchanged)),
        (_, true) => format!(
            "{} would be reformatted, {} already formatted",
            files(changed),
            files(unchanged)
        ),
        (_, false) => format!("{} reformatted, {} left unchanged", files(changed), files(unchanged)),
    }
}
