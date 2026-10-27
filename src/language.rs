use std::path::Path;

use serde::Deserialize;

/// A language spacify formats, as the `languages` setting names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
}

impl Language {
    pub const ALL: &[Self] = &[Self::Python];
}

/// What a file holds: the language, and the variant of it that changes the formatter's bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceKind {
    Python,
    /// A `.pyi` stub, where ruff allows one blank line at module level instead of two.
    PythonStub,
}

impl SourceKind {
    /// The kind a file's extension names, or `None` for an extension spacify does not know.
    #[must_use]
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "py" => Some(Self::Python),
            "pyi" => Some(Self::PythonStub),
            _ => None,
        }
    }

    #[must_use]
    pub fn language(self) -> Language {
        match self {
            Self::Python | Self::PythonStub => Language::Python,
        }
    }
}
