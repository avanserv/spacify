//! Vertical rhythm for source code: blank lines around logical blocks, inside the bounds that each
//! language's formatter keeps, so the two tools reach a fixed point in either order.

pub mod config;
mod edit;
mod gaps;
pub mod language;
mod python;
mod rules;

pub use config::Rules;
pub use gaps::FormatError;
pub use language::{Language, SourceKind};

/// Formats `source`, returning `None` when it is already formatted.
pub fn format_source(source: &str, kind: SourceKind, rules: &Rules) -> Result<Option<String>, FormatError> {
    if source.split("\r\n").any(|part| part.contains('\r')) {
        return Err(FormatError::LoneCarriageReturn);
    }

    match kind {
        SourceKind::Python => python::format(source, rules, false),
        SourceKind::PythonStub => python::format(source, rules, true),
    }
}
