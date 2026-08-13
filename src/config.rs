use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use log::{error, info};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Options {
    pub oob_done:           bool,
    pub trakt_enabled:      bool,
    pub punch_play_enabled: bool,
    pub tmdb_enabled:       bool,
    pub omdb_enabled:       bool,

    pub image_preload_rule: String,
}

#[derive(Default)]
pub struct Config {
    pub options: Options,

    // pub oob_stage: Option<u32>,
    home_dir: PathBuf,
}

impl Config {
    pub fn new(home_dir: &Path) -> Self {
        let mut s = Self {
            home_dir: home_dir.to_path_buf(),

            ..Default::default()
        };
        if home_dir.join("config.toml").is_file() {
            macro_rules! read_or_return {
                ($exp:expr, $err:expr) => {
                    match $exp {
                        Ok(val) => val,
                        Err(err) => {
                            error!($err, err);

                            let mut renamed = s.home_dir.join("corrupted_config.toml");
                            let mut i = 1;
                            while renamed.exists() {
                                renamed = s.home_dir.join(format!("corrupted_config_{i}.toml"));
                                i += 1;
                            }
                            _ = fs::rename(&s.home_dir.join("config.toml"), renamed);
                            _ = fs::write(
                                &s.home_dir.join("config.toml"),
                                toml::to_string_pretty(&s.options).unwrap(),
                            );

                            return s;
                        }
                    }
                };
            }

            let contents = read_or_return!(
                fs::read_to_string(home_dir.join("config.toml")),
                "Error while reading configuration: {}"
            );
            s.options = read_or_return!(
                toml::from_str(&contents),
                "Error while deserializing configuration: {}"
            );
        } else {
            info!("Config file not found, creating a new one..");
            _ = fs::write(
                s.home_dir.join("config.toml"),
                toml::to_string_pretty(&s.options).unwrap(),
            );
        }

        s
    }

    pub fn write_to_disk(&self) {
        let Some(err) = (|| -> anyhow::Result<()> {
            fs::rename(
                self.home_dir.join("config.toml"),
                self.home_dir.join("config.toml.bak"),
            )?;
            fs::write(
                self.home_dir.join("config.toml"),
                toml::to_string_pretty(&self.options)?,
            )
            .map_err(|err| anyhow!("{}", err))
        })()
        .err() else {
            return;
        };

        error!("Error while writing config to disk: {err}");
    }
}
