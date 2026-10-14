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
