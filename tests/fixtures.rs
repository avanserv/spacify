//! Each `tests/fixtures/<name>.py` formats to `<name>.expected.py`, and the expected file is a
//! fixed point. Set `SPACIFY_BLESS=1` to rewrite the expected files from the current output.

use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use spacify::{FormatError, Rules, format_source};

#[test]
fn fixtures() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let bless = std::env::var_os("SPACIFY_BLESS").is_some();
    let mut seen = 0;

    for entry in fs::read_dir(&dir).unwrap() {
        let input = entry.unwrap().path();
        let name = input.file_name().unwrap().to_string_lossy().into_owned();
        if name.contains(".expected.") {
            continue;
        }

        let (stem, ext) = name.rsplit_once('.').unwrap();
        let expected_path = dir.join(format!("{stem}.expected.{ext}"));
        let stub = ext == "pyi";
        let source = fs::read_to_string(&input).unwrap();
        let actual = format_source(&source, &Rules::default(), stub)
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .unwrap_or_else(|| source.clone());

        if bless {
            fs::write(&expected_path, &actual).unwrap();
        }
        let expected = fs::read_to_string(&expected_path).unwrap();
        assert_eq!(actual, expected, "{name}");
        assert_eq!(
            format_source(&expected, &Rules::default(), stub).unwrap(),
            None,
            "{name}: the expected output is not a fixed point"
        );
        seen += 1;
    }

    assert!(seen > 0, "no fixtures found in {}", dir.display());
}

#[test]
fn keeps_crlf_line_endings() {
    let source = "x = 1\r\nif x:\r\n    pass\r\n";
    let formatted = format_source(source, &Rules::default(), false).unwrap().unwrap();
    assert_eq!(formatted, "x = 1\r\n\r\nif x:\r\n    pass\r\n");
}

#[test]
fn keeps_a_missing_final_newline() {
    let source = "x = 1\nif x:\n    pass";
    let formatted = format_source(source, &Rules::default(), false).unwrap().unwrap();
    assert_eq!(formatted, "x = 1\n\nif x:\n    pass");
}

#[test]
fn reports_the_line_of_a_syntax_error() {
    let source = "x = 1\n\ndef f(:\n    pass\n";
    assert_eq!(
        format_source(source, &Rules::default(), false),
        Err(FormatError::Syntax(3))
    );
}

#[test]
fn leaves_blank_lines_inside_strings_alone() {
    let source = "x = \"\"\"\n\n\n\"\"\"\ny = 1\n";
    assert_eq!(format_source(source, &Rules::default(), false).unwrap(), None);
}

#[test]
fn each_rule_can_be_switched_off() {
    let off = Rules {
        blank_before_block: false,
        blank_after_block: false,
        blank_before_exit: false,
        collapse_groups: false,
        ..Rules::default()
    };
    let source = "def f(x):\n    x = 1\n\n    y = 2\n    if x:\n        pass\n    z = 3\n    return z\n";
    assert_eq!(format_source(source, &off, false).unwrap(), None);
}

#[test]
fn exit_threshold_is_configurable() {
    let rules = Rules {
        exit_min_block_statements: 2,
        ..Rules::default()
    };
    let source = "def f(x):\n    x = 1\n    return x\n";
    let formatted = format_source(source, &rules, false).unwrap().unwrap();
    assert_eq!(formatted, "def f(x):\n    x = 1\n\n    return x\n");
}
