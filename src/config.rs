use crate::toml_helper;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{env, fs};

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

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum ConfigError {
    #[error("I/O error")]
    IOError(#[from] std::io::Error),

    #[error("Failed to parse config")]
    InvalidConfig(#[from] conf::ConfigError),

    #[error("Failed to set `{key}` to `{value}` in config file")]
    SetError {
        key: String, // full name of the thing to set ("a.b", "a.b" will be the String)
        value: String,
        source: toml_helper::TomlKeyError,
    },

    #[error("Failed to get `{key}` in config file")]
    GetError {
        key: String,
        source: toml_helper::TomlKeyError,
    },

    #[error("Unrecognized key `{key}`, could not find a default value for it")]
    GetDefaultError {
        key: String,
        source: toml_helper::TomlKeyError,
    },

    #[error("Failed to parse config with toml_edit")]
    InvalidConfigEdit(#[from] toml_edit::TomlError),
}

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
    pub track: Track,
    pub editor: Editor,
    pub log: Log,

    #[serde(skip)]
    use_global: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Track {
    pub ignore: bool,
    pub ignore_git: bool,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            ignore: true,
            ignore_git: false,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "lowercase")]
pub enum ColorOptions {
    #[default]
    Auto,
    Always,
    Never,
}

impl FromStr for Config {
    type Err = conf::ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Config::load_str(s, false)
    }
}

impl Config {
    pub fn load(local_path: &Path, use_global: bool) -> Result<Self, conf::ConfigError> {
        Config::from_conf_source(
            conf::File::from(local_path)
                .format(conf::FileFormat::Toml)
                .required(false),
            use_global,
        )
    }

    pub fn load_str(config: &str, use_global: bool) -> Result<Self, conf::ConfigError> {
        Config::from_conf_source(
            conf::File::from_str(config, conf::FileFormat::Toml),
            use_global,
        )
    }

    fn from_conf_source<T>(source: T, use_global: bool) -> Result<Config, conf::ConfigError>
    where
        T: conf::Source + Send + Sync + 'static,
    {
        let mut builder = conf::Config::builder();
        if use_global && let Some(path) = Config::get_global_path() {
            builder = builder.add_source(
                conf::File::from(path)
                    .format(conf::FileFormat::Toml)
                    .required(false),
            );
        }
        builder = builder.add_source(source);

        let mut c: Config = builder.build()?.try_deserialize()?;
        c.use_global = use_global;
        Ok(c)
    }

    /// Get the path to the global config file, if it could be found
    /// This can return PathBuf even if the path does not exist
    /// Returns None if env var `FL_GLOBAL_CONFIG` is not set and [`env::home_dir`]` fails
    pub fn get_global_path() -> Option<PathBuf> {
        env::var_os("FL_GLOBAL_CONFIG")
            .map(|path| PathBuf::from(&path))
            .or_else(|| {
                env::home_dir().map(|home| home.join(".config").join("fl").join("config.toml"))
            })
    }
}

// Config file setters and getters
impl Config {
    pub fn set_key(
        &mut self,
        config_path: &Path,
        key: &str,
        value: &str,
    ) -> Result<(), ConfigError> {
        // parse value as toml value (like string or bool), and if that fails treat it as a string
        let val = value.parse().unwrap_or_else(|_| toml_edit::value(value));

        self.set_key_value(config_path, key, val)
    }

    pub fn set_key_value(
        &mut self,
        config_path: &Path,
        key: &str,
        value: toml_edit::Item,
    ) -> Result<(), ConfigError> {
        let config_content = fs::read_to_string(config_path)?;

        let mut doc: toml_edit::DocumentMut = config_content.parse()?;

        let err = toml_helper::set_key(&mut doc, key, value.clone());
        if let Err(e) = err {
            return Err(ConfigError::SetError {
                key: key.to_string(),
                value: value.to_string(),
                source: e,
            });
        };

        let new_content = doc.to_string();
        // make sure that the new config is valid
        Config::from_str(&new_content)?;
        // update self
        self.set_key_no_file(key, value.clone())?;

        fs::write(config_path, new_content)?;

        println!("Successfully updated config:");
        println!("{key} = {value}");
        Ok(())
    }

    pub fn set_key_default(&mut self, config_path: &Path, key: &str) -> Result<(), ConfigError> {
        let value = get_key_default_value(key)?;

        // this should always succeed, because key and value are both 100% valid
        // unless config_path doesn't exist, of course
        self.set_key_value(config_path, key, value)
    }

    pub fn get_key(&self, key: &str) -> Result<String, ConfigError> {
        // convert self to toml string, from which I get the key, so global is automatically handled
        let config_content = toml::to_string(self).expect("Config should alway be valid TOML");
        let doc: toml_edit::Document<_> = config_content.parse()?;

        let value = toml_helper::get_key(&doc, key).map_err(|e| ConfigError::GetError {
            key: key.to_string(),
            source: e,
        })?;

        Ok(value.to_string())
    }

    /// Set a key on this config, without changing any files
    pub fn set_key_no_file(
        &mut self,
        key: &str,
        value: toml_edit::Item,
    ) -> Result<(), ConfigError> {
        // convert self to toml string, so i only set the key that I want and nothing else
        let config_content = toml::to_string(self).expect("Config should alway be valid TOML");
        let value_str = value.to_string();

        let mut doc: toml_edit::DocumentMut = config_content.parse()?;
        toml_helper::set_key(&mut doc, key, value).map_err(|e| ConfigError::SetError {
            key: key.to_string(),
            value: value_str,
            source: e,
        })?;

        *self = doc.to_string().parse()?;

        Ok(())
    }

    pub fn unset_key(&mut self, config_path: &Path, key: &str) -> Result<(), ConfigError> {
        let config_content = fs::read_to_string(config_path)?;
        let mut doc: toml_edit::DocumentMut = config_content.parse()?;

        let from_where: &str;
        // get default value, this will also make sure the key is valid
        let value = if self.use_global
            && let Some(path) = Config::get_global_path()
            && let Ok(global_content) = fs::read_to_string(&path)
            && let Ok(value) = toml_helper::get_key(&global_content.parse()?, key)
        {
            from_where = "global";
            value
        } else {
            from_where = "default";
            get_key_default_value(key)?
        };

        // update self (in-memory)
        self.set_key_no_file(key, value.clone())?;

        // remove from file, I dont care if it exists in the file, because I know its a valid key at this point
        let _ = toml_helper::remove_key(&mut doc, key);

        fs::write(config_path, doc.to_string())?;

        println!("Successfully unset `{key}` (Now its `{value}`, from {from_where})");

        Ok(())
    }
}

pub fn get_key_default(key: &str) -> Result<String, ConfigError> {
    let value = get_key_default_value(key)?;
    Ok(value.to_string())
}

fn get_key_default_value(key: &str) -> Result<toml_edit::Item, ConfigError> {
    let doc = DEFAULT_CONFIG.parse()?;
    toml_helper::get_key(&doc, key).map_err(|e| ConfigError::GetDefaultError {
        key: key.to_string(),
        source: e,
    })
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
