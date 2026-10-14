use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;

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

pub fn is_python(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "py" || ext == "pyi")
}

/// The Python files under `paths`. A file named outright is kept even when excluded, unless
/// `force_exclude` is set, which is what a git hook wants since it names every staged file.
pub fn collect(paths: &[PathBuf], exclude: &Exclude, cwd: &Path, force_exclude: bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for path in paths {
        if !path.is_dir() {
            if !(force_exclude && exclude.matches(path, cwd)) {
                files.push(path.clone());
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
                .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()) && is_python(entry.path()))
                .map(ignore::DirEntry::into_path),
        );
    }

    files.sort();
    files.dedup();
    files
}
