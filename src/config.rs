use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

// All config options can be overridden via CLI arguments

/// Default toml file, includes comments explaining what each option does <br>
/// If you parse it, it will be same as [`Config::default`]
///
/// ```
/// use fl::config::DEFAULT_CONFIG;
/// use fl::config::Config;
/// use std::str::FromStr;
///
/// assert_eq!(
///     Config::from_str(DEFAULT_CONFIG).unwrap(),
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
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub color: ColorOptions,
    pub auto_update: bool,
    pub rm_commit_file: bool,
    pub editor: Editor,
    pub log: Log,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Editor {
    pub command: Vec<String>,
    pub ask_confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
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

impl FromStr for Config {
    type Err = conf::ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Config::load_str(s, false)
    }
}

impl Config {
    pub fn load(local_path: &Path, use_global: bool) -> Result<Self, conf::ConfigError> {
        let mut builder = conf::Config::builder();
        // load the global config first, only then load the local
        if use_global {
            builder = Self::handle_global(builder);
        }
        builder = builder.add_source(
            conf::File::from(local_path)
                // tell conf that this is toml, because if I dont it will break when file doesn't end with .toml
                .format(conf::FileFormat::Toml)
                .required(false),
        );

        builder.build()?.try_deserialize()
    }

    pub fn load_str(config: &str, use_global: bool) -> Result<Self, conf::ConfigError> {
        let mut builder = conf::Config::builder();
        if use_global {
            builder = Self::handle_global(builder);
        }
        builder = builder.add_source(conf::File::from_str(config, conf::FileFormat::Toml));
        builder.build()?.try_deserialize()
    }

    fn handle_global(
        builder: conf::ConfigBuilder<conf::builder::DefaultState>,
    ) -> conf::ConfigBuilder<conf::builder::DefaultState> {
        let global_config = env::var_os("FL_GLOBAL_CONFIG")
            .map(|path| PathBuf::from(&path))
            .or_else(|| {
                env::home_dir().map(|home| home.join(".config").join("fl").join("config.toml"))
            });

        if let Some(path) = global_config {
            builder.add_source(
                conf::File::from(path)
                    .format(conf::FileFormat::Toml)
                    .required(false),
            )
        } else {
            builder
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
        assert_eq!(Config::default(), Config::from_str(DEFAULT_CONFIG).unwrap())
    }

    /// make sure that [`DEFAULT_CONFIG`] explicitly defines all options
    #[test]
    fn test_default_is_same() {
        let default_toml: toml::Value = toml::from_str(DEFAULT_CONFIG).unwrap();
        let generated_toml = toml::Value::try_from(Config::default()).unwrap();

        assert_eq!(
            default_toml, generated_toml,
            "DEFAULT_CONFIG is not up to date"
        );
    }
}
