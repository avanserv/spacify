use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::language::Language;

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

/// One language's changes to the shared rules: a key left out keeps the shared value.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct RuleOverrides {
    pub blank_before_block: Option<bool>,
    pub blank_after_block: Option<bool>,
    pub blank_before_exit: Option<bool>,
    pub exit_min_block_statements: Option<usize>,
    pub collapse_groups: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct LanguageConfig {
    pub rules: RuleOverrides,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Replaces [`DEFAULT_EXCLUDE`] when set.
    pub exclude: Option<Vec<String>>,
    pub extend_exclude: Vec<String>,
    pub languages: Vec<Language>,
    /// Shared by every language, under its own overrides.
    pub rules: Rules,
    pub python: LanguageConfig,
    pub rust: LanguageConfig,
    /// The directory exclusion patterns are relative to. Filled in on load.
    #[serde(skip)]
    pub root: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exclude: None,
            extend_exclude: Vec::new(),
            languages: Language::ALL.to_vec(),
            rules: Rules::default(),
            python: LanguageConfig::default(),
            rust: LanguageConfig::default(),
            root: PathBuf::new(),
        }
    }
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

    #[must_use]
    pub fn is_enabled(&self, language: Language) -> bool {
        self.languages.contains(&language)
    }

    /// The shared rules with `language`'s overrides on top.
    #[must_use]
    pub fn rules_for(&self, language: Language) -> Rules {
        let overrides = match language {
            Language::Python => &self.python.rules,
            Language::Rust => &self.rust.rules,
        };
        let shared = &self.rules;
        Rules {
            blank_before_block: overrides.blank_before_block.unwrap_or(shared.blank_before_block),
            blank_after_block: overrides.blank_after_block.unwrap_or(shared.blank_after_block),
            blank_before_exit: overrides.blank_before_exit.unwrap_or(shared.blank_before_exit),
            exit_min_block_statements: overrides
                .exit_min_block_statements
                .unwrap_or(shared.exit_min_block_statements),
            collapse_groups: overrides.collapse_groups.unwrap_or(shared.collapse_groups),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_is_enabled_by_default() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.languages, Language::ALL);
    }

    #[test]
    fn a_language_override_wins_over_the_shared_rule() {
        let config: Config = toml::from_str(
            "[rules]\nexit-min-block-statements = 4\ncollapse-groups = false\n\n\
             [python.rules]\nexit-min-block-statements = 2\n",
        )
        .unwrap();
        let rules = config.rules_for(Language::Python);
        assert_eq!(rules.exit_min_block_statements, 2);
        assert!(!rules.collapse_groups);
        assert!(rules.blank_before_block);
    }

    #[test]
    fn rejects_an_unknown_language() {
        assert!(toml::from_str::<Config>("languages = [\"cobol\"]\n").is_err());
    }
}
