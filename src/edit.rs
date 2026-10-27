/// A change to the blank-line run that ends just above `row`.
#[derive(Debug, Clone, Copy)]
pub struct Edit {
    pub row: usize,
    pub current: usize,
    pub desired: usize,
}

/// Applies `edits` to `source` line by line. Every byte outside the edited blank runs is copied
/// verbatim, which is what lets the rules ignore everything inside a statement.
pub fn apply(source: &str, edits: &mut [Edit]) -> String {
    edits.sort_unstable_by_key(|edit| edit.row);
    let newline = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(source.len() + edits.len() * newline.len());
    let mut pending = edits.iter().peekable();
    let mut skip_until = 0;

    for (row, line) in source.split_inclusive('\n').enumerate() {
        if let Some(edit) = pending.peek()
            && edit.row - edit.current == row
        {
            // The run starts on this row: drop the surplus here, or add the missing lines here.
            if edit.desired < edit.current {
                skip_until = row + (edit.current - edit.desired);
            } else {
                for _ in edit.current..edit.desired {
                    out.push_str(newline);
                }
            }
            pending.next();
        }

        if row >= skip_until {
            out.push_str(line);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(row: usize, current: usize, desired: usize) -> Edit {
        Edit { row, current, desired }
    }

    #[test]
    fn inserts_into_an_empty_run() {
        assert_eq!(apply("a\nb\n", &mut [edit(1, 0, 1)]), "a\n\nb\n");
    }

    #[test]
    fn shrinks_a_run_to_nothing() {
        assert_eq!(apply("a\n\n\nb\n", &mut [edit(3, 2, 0)]), "a\nb\n");
    }

    #[test]
    fn shrinks_a_run_from_its_top() {
        assert_eq!(apply("a\n\n  \nb\n", &mut [edit(3, 2, 1)]), "a\n  \nb\n");
    }

    #[test]
    fn applies_edits_given_out_of_order() {
        let mut edits = [edit(4, 1, 0), edit(1, 0, 1)];
        assert_eq!(apply("a\nb\nc\n\nd\n", &mut edits), "a\n\nb\nc\nd\n");
    }

    #[test]
    fn inserts_crlf_into_a_crlf_file() {
        assert_eq!(apply("a\r\nb\r\n", &mut [edit(1, 0, 2)]), "a\r\n\r\n\r\nb\r\n");
    }

    #[test]
    fn edits_above_a_last_line_without_a_newline() {
        assert_eq!(apply("a\n\n\nb", &mut [edit(3, 2, 1)]), "a\n\nb");
        assert_eq!(apply("a\nb", &mut [edit(1, 0, 1)]), "a\n\nb");
    }
}
