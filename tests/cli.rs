use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const UNFORMATTED: &str = "x = 1\nif x:\n    pass\n";
const FORMATTED: &str = "x = 1\n\nif x:\n    pass\n";
const UNFORMATTED_RS: &str = "fn f(x: u32) {\n    let y = x;\n    if y > 0 {\n        g();\n    }\n}\n";
const FORMATTED_RS: &str = "fn f(x: u32) {\n    let y = x;\n\n    if y > 0 {\n        g();\n    }\n}\n";

fn spacify(dir: &TempDir, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_spacify"))
        .args(args)
        .current_dir(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = stdin {
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    }
    child.wait_with_output().unwrap()
}

fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    dir
}

#[test]
fn formats_in_place() {
    let dir = project(&[("a.py", UNFORMATTED)]);
    let output = spacify(&dir, &[], None);
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(dir.path().join("a.py")).unwrap(), FORMATTED);
}

#[test]
fn check_fails_without_writing() {
    let dir = project(&[("a.py", UNFORMATTED)]);
    let output = spacify(&dir, &["--check"], None);
    assert_eq!(output.status.code(), Some(1));
    // The path keeps the platform's separator, `.\a.py` on Windows.
    assert!(String::from_utf8_lossy(&output.stdout).contains("Would reformat: ."));
    assert!(String::from_utf8_lossy(&output.stdout).contains("a.py"));
    assert_eq!(fs::read_to_string(dir.path().join("a.py")).unwrap(), UNFORMATTED);
}

#[test]
fn check_passes_on_formatted_code() {
    let dir = project(&[("a.py", FORMATTED)]);
    assert!(spacify(&dir, &["--check"], None).status.success());
}

#[test]
fn diff_prints_a_unified_diff() {
    let dir = project(&[("a.py", UNFORMATTED)]);
    let output = spacify(&dir, &["--diff", "a.py"], None);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("+\n"));
}

#[test]
fn formats_stdin() {
    let dir = project(&[]);
    let output = spacify(&dir, &["-"], Some(UNFORMATTED));
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), FORMATTED);
}

#[test]
fn syntax_error_exits_2() {
    let dir = project(&[("bad.py", "def f(:\n")]);
    let output = spacify(&dir, &[], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("syntax error on line 1"));
}

#[test]
fn lone_carriage_returns_exit_2_untouched() {
    let source = "x = 1\rif x:\r    pass\r";
    let dir = project(&[("a.py", source)]);
    let output = spacify(&dir, &[], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("bare carriage return"));
    assert_eq!(fs::read_to_string(dir.path().join("a.py")).unwrap(), source);
}

#[test]
fn skips_default_excludes_when_walking() {
    let dir = project(&[(".venv/lib/a.py", UNFORMATTED), ("build/b.py", UNFORMATTED)]);
    let output = spacify(&dir, &["--check"], None);
    // `.venv` is a default exclude, `build` is not.
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("b.py"));
    assert!(!stdout.contains(".venv"));
}

#[test]
fn force_exclude_skips_named_files() {
    let dir = project(&[
        ("pyproject.toml", "[tool.spacify]\nextend-exclude = [\"generated\"]\n"),
        ("generated/a.py", UNFORMATTED),
    ]);
    assert_eq!(
        spacify(&dir, &["--check", "generated/a.py"], None).status.code(),
        Some(1)
    );
    assert!(
        spacify(&dir, &["--check", "--force-exclude", "generated/a.py"], None)
            .status
            .success()
    );
}

#[test]
fn reads_rules_from_pyproject() {
    let dir = project(&[
        ("pyproject.toml", "[tool.spacify.rules]\nblank-before-block = false\n"),
        ("a.py", UNFORMATTED),
    ]);
    assert!(spacify(&dir, &["--check"], None).status.success());
}

#[test]
fn leaves_a_disabled_language_alone() {
    let dir = project(&[("spacify.toml", "languages = []\n"), ("a.py", UNFORMATTED)]);
    assert!(spacify(&dir, &["--check"], None).status.success());
    assert!(spacify(&dir, &["--check", "a.py"], None).status.success());
    let output = spacify(&dir, &["-"], Some(UNFORMATTED));
    assert_eq!(String::from_utf8_lossy(&output.stdout), UNFORMATTED);
}

#[test]
fn formats_rust_by_default() {
    let dir = project(&[("a.py", FORMATTED), ("src/lib.rs", UNFORMATTED_RS)]);
    assert!(spacify(&dir, &[], None).status.success());
    assert_eq!(fs::read_to_string(dir.path().join("src/lib.rs")).unwrap(), FORMATTED_RS);
}

#[test]
fn leaves_rust_alone_when_only_python_is_enabled() {
    let dir = project(&[
        ("spacify.toml", "languages = [\"python\"]\n"),
        ("a.py", UNFORMATTED),
        ("lib.rs", UNFORMATTED_RS),
    ]);
    assert!(spacify(&dir, &[], None).status.success());
    assert_eq!(fs::read_to_string(dir.path().join("a.py")).unwrap(), FORMATTED);
    assert_eq!(fs::read_to_string(dir.path().join("lib.rs")).unwrap(), UNFORMATTED_RS);
}

#[test]
fn formats_stdin_as_the_language_its_name_says() {
    let dir = project(&[]);
    let output = spacify(&dir, &["-", "--stdin-filename", "lib.rs"], Some(UNFORMATTED_RS));
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), FORMATTED_RS);
}

#[test]
fn applies_rust_rule_overrides_to_rust_only() {
    let dir = project(&[
        ("spacify.toml", "[rust.rules]\nblank-before-block = false\n"),
        ("a.py", UNFORMATTED),
        ("lib.rs", UNFORMATTED_RS),
    ]);
    assert_eq!(spacify(&dir, &["--check"], None).status.code(), Some(1));
    assert!(spacify(&dir, &["--check", "lib.rs"], None).status.success());
}

#[test]
fn reads_language_rule_overrides() {
    let dir = project(&[
        (
            "pyproject.toml",
            "[tool.spacify.rules]\nblank-before-block = false\n\n[tool.spacify.python.rules]\nblank-before-block = true\n",
        ),
        ("a.py", UNFORMATTED),
    ]);
    assert_eq!(spacify(&dir, &["--check"], None).status.code(), Some(1));
}

#[test]
fn rejects_unknown_config_keys() {
    let dir = project(&[("spacify.toml", "[rules]\nno-such-rule = true\n"), ("a.py", FORMATTED)]);
    let output = spacify(&dir, &[], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no-such-rule"));
}
