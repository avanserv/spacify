use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

use spacify::SourceKind;
use spacify::config::Config;

/// Exclusion patterns as ruff reads them: one with a `/` matches the path relative to the
/// configuration's directory, any other matches a single path component.
#[derive(Clone)]
pub struct Exclude {
    root: PathBuf,
    paths: GlobSet,
    names: GlobSet,
}

impl Exclude {
    pub fn new(config: &Config, cwd: &Path) -> Result<Self> {
        let mut paths = GlobSetBuilder::new();
        let mut names = GlobSetBuilder::new();
        for pattern in config.exclude_patterns() {
            let glob = Glob::new(pattern.trim_end_matches('/'))
                .with_context(|| format!("invalid exclude pattern {pattern:?}"))?;
            if pattern.trim_end_matches('/').contains('/') {
                paths.add(glob);
            } else {
                names.add(glob);
            }
        }

        Ok(Self {
            root: cwd.join(&config.root),
            paths: paths.build()?,
            names: names.build()?,
        })
    }

    pub fn matches(&self, path: &Path, cwd: &Path) -> bool {
        let absolute = cwd.join(path);
        let relative = absolute.strip_prefix(&self.root).unwrap_or(path);
        self.paths.is_match(relative)
            || relative
                .components()
                .any(|component| self.names.is_match(component.as_os_str()))
    }
}

/// The kind of a file `collect` keeps, or `None` to leave it out: the language must be enabled. A file
/// named outright with an unknown extension is formatted as Python, as it always has been.
fn kind_of(path: &Path, config: &Config, named: bool) -> Option<SourceKind> {
    let kind = match SourceKind::from_path(path) {
        Some(kind) => kind,
        None if named => SourceKind::Python,
        None => return None,
    };
    config.is_enabled(kind.language()).then_some(kind)
}

/// The files under `paths` in an enabled language. A file named outright is kept even when excluded,
/// unless `force_exclude` is set, which is what a git hook wants since it names every staged file.
pub fn collect(
    paths: &[PathBuf],
    config: &Config,
    exclude: &Exclude,
    cwd: &Path,
    force_exclude: bool,
) -> Vec<(PathBuf, SourceKind)> {
    let mut files = Vec::new();
    for path in paths {
        if !path.is_dir() {
            if !(force_exclude && exclude.matches(path, cwd))
                && let Some(kind) = kind_of(path, config, true)
            {
                files.push((path.clone(), kind));
            }
            continue;
        }
        // cheaper than any indirection on the per-entry check.
        let (filter, root) = (exclude.clone(), cwd.to_path_buf());
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .filter_entry(move |entry| !filter.matches(entry.path(), &root))
            .build();
        files.extend(
            walker
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
                .filter_map(|entry| {
                    let kind = kind_of(entry.path(), config, false)?;
                    Some((entry.into_path(), kind))
                }),
        );
    }

    files.sort();
    files.dedup();
    files
}
