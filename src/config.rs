use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// The directories ruff skips by default, so that both tools see the same tree.
pub const DEFAULT_EXCLUDE: &[&str] = &[
    ".bzr",
    ".direnv",
    ".eggs",
    ".git",
    ".git-rewrite",
    ".hg",
    ".ipynb_checkpoints",
    ".mypy_cache",
    ".nox",
    ".pants.d",
    ".pyenv",
    ".pytest_cache",
    ".pytype",
    ".ruff_cache",
    ".svn",
    ".tox",
    ".venv",
    ".vscode",
    "__pypackages__",
    "_build",
    "buck-out",
    "dist",
    "node_modules",
    "site-packages",
    "venv",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Rules {
    pub blank_before_block: bool,
    pub blank_after_block: bool,
    pub blank_before_exit: bool,
    /// A block shorter than this keeps its exit statement tight against the line before it.
    pub exit_min_block_statements: usize,
    pub collapse_groups: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            blank_before_block: true,
            blank_after_block: true,
            blank_before_exit: true,
            exit_min_block_statements: 3,
            collapse_groups: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Replaces [`DEFAULT_EXCLUDE`] when set.
    pub exclude: Option<Vec<String>>,
    pub extend_exclude: Vec<String>,
    pub rules: Rules,
    /// The directory exclusion patterns are relative to. Filled in on load.
    #[serde(skip)]
    pub root: PathBuf,
}

#[derive(Deserialize)]
struct Pyproject {
    tool: Option<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    spacify: Option<Config>,
}

impl Config {
    /// The first of `spacify.toml`, `.spacify.toml` or a `pyproject.toml` with `[tool.spacify]`
    /// found from `start` upwards, or the defaults rooted at `start`.
    pub fn discover(start: &Path) -> Result<Self> {
        for dir in start.ancestors() {
            for name in ["spacify.toml", ".spacify.toml"] {
                let path = dir.join(name);
                if path.is_file() {
                    return Self::load(&path);
                }
            }

            let pyproject = dir.join("pyproject.toml");
            if pyproject.is_file()
                && let Some(config) = Self::from_pyproject(&pyproject)?
            {
                return Ok(config);
            }
        }

        Ok(Self {
            root: start.to_path_buf(),
            ..Self::default()
        })
    }

    pub fn load(path: &Path) -> Result<Self> {
        if path.file_name().is_some_and(|name| name == "pyproject.toml") {
            return Self::from_pyproject(path)?
                .with_context(|| format!("{} has no [tool.spacify] table", path.display()));
        }

        let text = fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        let mut config: Self = toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))?;
        config.root = parent_dir(path);
        Ok(config)
    }

    fn from_pyproject(path: &Path) -> Result<Option<Self>> {
        let text = fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        let pyproject: Pyproject = toml::from_str(&text).with_context(|| format!("invalid {}", path.display()))?;
        Ok(pyproject.tool.and_then(|tool| tool.spacify).map(|mut config| {
            config.root = parent_dir(path);
            config
        }))
    }

    pub fn exclude_patterns(&self) -> Vec<String> {
        let base = match &self.exclude {
            Some(patterns) => patterns.clone(),
            None => DEFAULT_EXCLUDE.iter().map(ToString::to_string).collect(),
        };
        base.into_iter().chain(self.extend_exclude.iter().cloned()).collect()
    }
}

fn parent_dir(path: &Path) -> PathBuf {
    path.parent().map_or_else(PathBuf::new, Path::to_path_buf)
}
