use std::{fs, sync::LazyLock};

use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use systemd::sd_journal_log;

const VRAM_BOOST_DEFAULT: bool = true;
const RAM_BOOST_DEFAULT: bool = false;
const CPU_BOOST_DEFAULT: bool = false;
const VRAM_BOOST_VALUE_DEFAULT: f32 = 100.0;
const RAM_BOOST_VALUE_DEFAULT: f32 = 100.0;
const CPU_BOOST_VALUE_DEFAULT: u16 = 10_000;

#[serde_inline_default]
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde_inline_default(VRAM_BOOST_DEFAULT)]
    pub vram_boost: bool,
    #[serde_inline_default(CPU_BOOST_DEFAULT)]
    pub cpu_boost: bool,
    #[serde_inline_default(RAM_BOOST_DEFAULT)]
    pub ram_boost: bool,
    #[serde_inline_default(VRAM_BOOST_VALUE_DEFAULT)]
    pub vram_boost_value: f32,
    #[serde_inline_default(RAM_BOOST_VALUE_DEFAULT)]
    pub ram_boost_value: f32,
    #[serde_inline_default(CPU_BOOST_VALUE_DEFAULT)]
    pub cpu_boost_value: u16,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            vram_boost: VRAM_BOOST_DEFAULT,
            cpu_boost: CPU_BOOST_DEFAULT,
            ram_boost: RAM_BOOST_DEFAULT,
            vram_boost_value: VRAM_BOOST_VALUE_DEFAULT,
            ram_boost_value: RAM_BOOST_VALUE_DEFAULT,
            cpu_boost_value: CPU_BOOST_VALUE_DEFAULT,
        }
    }
}
impl Config {
    fn limit(mut self) -> Self {
        if !self.vram_boost_value.is_finite()
            || self.vram_boost_value < 0.0
            || 100.0 < self.vram_boost_value
        {
            sd_journal_log!(
                4,
                "vram_boost_value must be between 0.0 and 100.0 inclusive. Setting to 100.0"
            );
            self.vram_boost_value = 100.0;
        }
        if self.cpu_boost_value < 100 || 10_000 < self.cpu_boost_value {
            sd_journal_log!(
                4,
                "cpu_boost_value must be between 100 and 10000 inclusive. Setting to 10000"
            );
            self.cpu_boost_value = 10_000;
        }
        if !self.ram_boost_value.is_finite()
            || self.ram_boost_value < 0.0
            || 100.0 < self.ram_boost_value
        {
            sd_journal_log!(
                4,
                "ram_boost_value must be between 0.0 and 100.0 inclusive. Setting to 100.0"
            );
            self.ram_boost_value = 100.0;
        }
        self
    }
}

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let path = xdg::BaseDirectories::new().find_config_file("hyprland-focused-booster.toml");
    if let Some(config_path) = path {
        let config_file = fs::read_to_string(config_path);
        if let Ok(config) = config_file {
            let toml = basic_toml::from_str::<Config>(&config);
            toml.unwrap_or_else(|e| {
                sd_journal_log!(3, "Error {e} when reading config. Using default values");
                Config::default()
            })
            .limit()
        } else {
            Config::default()
        }
    } else {
        Config::default()
    }
});
