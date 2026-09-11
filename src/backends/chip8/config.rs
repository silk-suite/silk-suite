use super::metadata::Platform;
use crate::core::{Config, ConfigEntry, ConfigValue};

#[derive(Clone)]
pub struct CHIP8Config {
    pub entries: Config,
}

impl CHIP8Config {
    pub fn new() -> Self {
        let platform_override = ConfigEntry::new(
            "Platform Override".to_string(),
            ConfigValue::Choice {
                selected: 999,
                options: Platform::as_vec(),
            },
        );
        Self {
            entries: vec![platform_override],
        }
    }
}
