use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::PathBuf,
};

use directories::ProjectDirs;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};

pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 200.0;
pub const MAX_TOTAL_BEATS: u32 = 12;
pub const MIN_TOTAL_BEATS: u32 = 2;
pub const MAX_VOLUME: f64 = 1.0; // a hack for float precision issue
pub const MIN_VOLUME: f64 = 0.0;
// pub const PRECISION: u32 = 2;

#[derive(Serialize, Deserialize)]
pub struct CoryConfig {
    pub bpm: f64,
    pub volume: f64,
}

impl Default for CoryConfig {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            volume: 1.0,
        }
    }
}

impl CoryConfig {
    #[allow(dead_code)]
    pub fn new(bpm: f64, volume: f64) -> Self {
        Self { bpm, volume }
    }

    pub fn load() -> Result<Self> {
        let config_path = get_config_path()?;
        match File::open(config_path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                let config: CoryConfig = match serde_json::from_reader(reader) {
                    Ok(x) => x,
                    Err(_) => Self::default(),
                };
                Ok(config.to_rounded())
            }
            Err(_) => Ok(Self::default()),
        }
    }

    pub fn write(&self) -> Result<()> {
        let json_str = serde_json::to_string(&self.to_rounded())?;
        let config_path = get_config_path()?;
        if let Some(parent_dir) = config_path.parent() {
            fs::create_dir_all(parent_dir)?;
        }
        let mut file = File::create(config_path)?;
        file.write_all(json_str.as_bytes())?;
        Ok(())
    }

    fn to_rounded(&self) -> Self {
        Self {
            bpm: self.bpm.clamp(MIN_BPM, MAX_BPM),
            volume: self.volume.clamp(MIN_VOLUME, MAX_VOLUME),
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let mut directory = if let Ok(s) = std::env::var("CORY_CONFIG") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "yz", "cory") {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        return Err(eyre!(
            "Unable to find config directory for ratatui-template"
        ));
    };

    directory.push("config.json");
    Ok(directory)
}
