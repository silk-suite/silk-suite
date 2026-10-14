use crate::core::{Config, ConfigEntry, ConfigValue};

#[derive(Clone)]
pub struct NESConfig {
    pub entries: Config,
}

impl NESConfig {
    pub fn new() -> Self {
        let placeholder = ConfigEntry::new(
            "Placeholder".to_string(),
            ConfigValue::Integer { raw: 0, range: (0, 10) },
        );
        Self {
            entries: vec![placeholder],
        }
    }
}
