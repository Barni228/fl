use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, IsTerminal};

// All config options can be overridden via CLI arguments

/// Default toml file, includes comments explaining what each option does <br>
/// If you parse it, it will be same as [`Config::default`]
///
/// ```
/// use fl::config::DEFAULT_CONFIG;
/// use fl::config::Config;
///
/// assert_eq!(
///     toml::from_str::<Config>(DEFAULT_CONFIG).unwrap(),
///     Config::default()
/// )
/// ```
pub const DEFAULT_CONFIG: &str = include_str!("../default_config.toml");

// Types

#[derive(thiserror::Error, Debug)]
pub enum BetterEnvError {
    // source is a special name, thiserror will use it for .source() method
    #[error("Failed to get env var `{var}`")]
    EnvVarError { var: String, source: env::VarError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub colors: ColorOptions,
    pub rm_commit_file: bool,
    pub editor: Editor,
    pub log: Log,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    pub command: Vec<String>,
    pub ask_confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Log {
    pub max: u32,
    pub print_title: bool,
    pub print_title_quotes: bool,
    pub print_number_of_changes: bool,
    pub print_time_ago: bool,
    pub print_date: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "lowercase")]
pub enum ColorOptions {
    #[default]
    Auto,
    Always,
    Never,
}

// Defaults

impl Default for Log {
    fn default() -> Self {
        Self {
            max: 0,
            print_title: true,
            print_title_quotes: false,
            print_number_of_changes: false,
            print_time_ago: true,
            print_date: false,
        }
    }
}

impl Config {
    pub fn use_color(&self) -> bool {
        match self.colors {
            ColorOptions::Auto => io::stdout().is_terminal(),
            ColorOptions::Always => true,
            ColorOptions::Never => false,
        }
    }
}

// Helpers
impl Editor {
    pub fn editor(&self) -> Result<String, BetterEnvError> {
        // if command has at least something, use it
        if let Some(first) = self.command.first() {
            Editor::handle_env(first)
        // if command is empty, try to use $EDITOR
        } else if let Ok(editor) = env::var("EDITOR") {
            Ok(editor)
        // if $EDITOR is not set, default to vim
        } else {
            Ok("vim".to_string())
        }
    }

    pub fn args(&self) -> Result<Vec<String>, BetterEnvError> {
        self.command
            .iter()
            .skip(1) // skip the first element, which is the editor
            .map(|s| Editor::handle_env(s))
            .collect()
    }

    fn handle_env(s: &str) -> Result<String, BetterEnvError> {
        if let Some(env_var) = s.strip_prefix('$') {
            env::var(env_var).map_err(|_| BetterEnvError::EnvVarError {
                var: env_var.to_string(),
                source: env::VarError::NotPresent,
            })
        } else {
            Ok(s.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        assert_eq!(
            toml::from_str::<Config>(DEFAULT_CONFIG).unwrap(),
            Config::default()
        )
    }
}
