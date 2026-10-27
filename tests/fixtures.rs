//! Each `tests/fixtures/<name>.<ext>` formats to `<name>.expected.<ext>`, and the expected file is a
//! fixed point. Set `SPACIFY_BLESS=1` to rewrite the expected files from the current output.

use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use spacify::{FormatError, Rules, SourceKind, format_source};

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
        let kind = SourceKind::from_path(&input).unwrap();
        let source = fs::read_to_string(&input).unwrap();
        let actual = format_source(&source, kind, &Rules::default())
            .unwrap_or_else(|error| panic!("{name}: {error}"))
            .unwrap_or_else(|| source.clone());

        if bless {
            fs::write(&expected_path, &actual).unwrap();
        }
        let expected = fs::read_to_string(&expected_path).unwrap();
        assert_eq!(actual, expected, "{name}");
        assert_eq!(
            format_source(&expected, kind, &Rules::default()).unwrap(),
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
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "x = 1\r\n\r\nif x:\r\n    pass\r\n");
}

#[test]
fn keeps_a_missing_final_newline() {
    let source = "x = 1\nif x:\n    pass";
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "x = 1\n\nif x:\n    pass");
}

#[test]
fn reports_the_line_of_a_syntax_error() {
    let source = "x = 1\n\ndef f(:\n    pass\n";
    assert_eq!(
        format_source(source, SourceKind::Python, &Rules::default()),
        Err(FormatError::Syntax(3))
    );
}

#[test]
fn leaves_blank_lines_inside_strings_alone() {
    let source = "x = \"\"\"\n\n\n\"\"\"\ny = 1\n";
    assert_eq!(
        format_source(source, SourceKind::Python, &Rules::default()).unwrap(),
        None
    );
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
    assert_eq!(format_source(source, SourceKind::Python, &off).unwrap(), None);
}

#[test]
fn exit_threshold_is_configurable() {
    let rules = Rules {
        exit_min_block_statements: 2,
        ..Rules::default()
    };
    let source = "def f(x):\n    x = 1\n    return x\n";
    let formatted = format_source(source, SourceKind::Python, &rules).unwrap().unwrap();
    assert_eq!(formatted, "def f(x):\n    x = 1\n\n    return x\n");
}

#[test]
fn keeps_a_byte_order_mark() {
    let source = "\u{feff}x = 1\nif x:\n    pass\n";
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "\u{feff}x = 1\n\nif x:\n    pass\n");
}

#[test]
fn indents_with_tabs() {
    let source = "def f(x):\n\ty = 1\n\tif y:\n\t\tpass\n\treturn y\n";
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "def f(x):\n\ty = 1\n\n\tif y:\n\t\tpass\n\n\treturn y\n");
}

/// Ruff drops a form feed and keeps its line as a blank one, so spacify counts it as blank too.
#[test]
fn counts_a_form_feed_line_as_blank() {
    let source = "x = 1\n\x0c\nif x:\n    pass\n";
    assert_eq!(
        format_source(source, SourceKind::Python, &Rules::default()).unwrap(),
        None
    );
    let source = "x = 1\n\x0cif x:\n    pass\n";
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "x = 1\n\n\x0cif x:\n    pass\n");
}

#[test]
fn leaves_empty_and_comment_only_files_alone() {
    for source in ["", "\n", "# a\n\n\n# b\n"] {
        assert_eq!(
            format_source(source, SourceKind::Python, &Rules::default()).unwrap(),
            None,
            "{source:?}"
        );
    }
}

/// Tree-sitter reads this as a Python 2 print statement, but it is a valid Python 3 tuple, so
/// spacify formats around it rather than refusing it.
#[test]
fn formats_around_a_print_chevron() {
    let source = "print >>f, \"a\"\nif f:\n    pass\n";
    let formatted = format_source(source, SourceKind::Python, &Rules::default())
        .unwrap()
        .unwrap();
    assert_eq!(formatted, "print >>f, \"a\"\n\nif f:\n    pass\n");
}

/// Valid since Python 3.13 (PEP 696), but tree-sitter-python rejects it. A grammar upgrade that
/// fails this test lifts the limit: update `README.md` and the Makefile's `CPYTHON_SKIP`.
#[test]
fn refuses_type_parameter_defaults() {
    for source in [
        "def f[T = int](): pass\n",
        "class C[T = int]: pass\n",
        "type A[T = int] = T\n",
    ] {
        assert_eq!(
            format_source(source, SourceKind::Python, &Rules::default()),
            Err(FormatError::Syntax(1)),
            "{source:?}"
        );
    }
}

#[test]
fn refuses_lone_carriage_returns() {
    for source in ["x = 1\rif x:\r    pass\r", "x = 1\r\ny = '''\r'''\r\n"] {
        assert_eq!(
            format_source(source, SourceKind::Python, &Rules::default()),
            Err(FormatError::LoneCarriageReturn),
            "{source:?}"
        );
    }
}
