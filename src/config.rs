use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Options {
    pub image_preload_rule: String,
    pub oob_done:           bool,
    pub oob_stage:          Option<u32>,
    pub trakt_enabled:      bool,
    pub tmdb_enabled:       bool,
    pub omdb_enabled:       bool,
}
impl Options {
    pub fn default() -> Self {
        Self {
            image_preload_rule: "all".into(),
            oob_done:           false,
            oob_stage:          None,
            trakt_enabled:      true,
            tmdb_enabled:       true,
            omdb_enabled:       true,
        }
    }
}

pub struct Config {
    pub options: Options,

    home_dir: PathBuf,
}

impl Config {
    pub fn new(home_dir: &PathBuf) -> anyhow::Result<Self> {
        let mut s = Self {
            options:  Options::default(),
            home_dir: home_dir.clone(),
        };
        if home_dir.join("config.toml").is_file() {
            macro_rules! read_or_return {
                ($exp:expr, $err:expr) => {
                    match $exp {
                        Ok(val) => val,
                        Err(err) => {
                            // error!($err, err);

                            // let mut renamed = s.home_dir.join("corrupted_config.toml");
                            // let mut i = 1;
                            // while renamed.exists() {
                            //     renamed = s.home_dir.join(format!("corrupted_config_{i}.toml"));
                            //     i += 1;
                            // }
                            // fs::rename(&s.home_dir.join("config.toml"), renamed)?;
                            // fs::write(&s.home_dir.join("config.toml"), toml::to_string_pretty(s.options)?)?;

                            return Ok(s);
                        }
                    }
                };
            }

            let contents = read_or_return!(
                fs::read_to_string(home_dir.join("config.toml")),
                "Error while reading configuration: {}"
            );
            // s.options = read_or_return!(toml::from_str(&contents), "Error while deserializing configuration: {}");
        } else {
            info!("Config file not found, creating a new one..");
            // fs::write(&s.home_dir.join("config.toml"), toml::to_string_pretty(s.options)?)?;
        }

        Ok(s)
    }

    pub fn write_to_disk(&self) -> anyhow::Result<()> {
        // fs::rename(&s.home_dir.join("config.toml"), "config.toml.bak")?;
        // fs::write(&s.home_dir.join("config.toml"), toml::to_string_pretty(s.options)?)?;
        Ok(())
    }
}
