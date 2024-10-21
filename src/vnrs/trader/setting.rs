use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

pub static SETTINGS: OnceLock<HashMap<&'static str, String>> = OnceLock::new();

pub fn get_settings() -> &'static HashMap<&'static str, String> {
    SETTINGS.get_or_init(|| {
        [
            ("font.family", "微软雅黑".to_string()),
            ("font.size", 12.to_string()),
            ("log.active", "True".to_string()),
            ("log.level", "CRITICAL".to_string()),
            ("log.console", "True".to_string()),
            ("log.file", "True".to_string()),
            ("email.server", "smtp.qq.com".to_string()),
            ("email.port", "465".to_string()),
            ("email.username", "".to_string()),
            ("email.password", "".to_string()),
            ("email.sender", "".to_string()),
            ("email.receiver", "".to_string()),
            ("datafeed.name", "".to_string()),
            ("datafeed.username", "".to_string()),
            ("datafeed.password", "".to_string()),
            ("database.timezone", "LOCAL".to_string()),
            ("database.name", "sqlite".to_string()),
            ("database.database", "database.db".to_string()),
            ("database.host", "".to_string()),
            ("database.port", 0.to_string()),
            ("database.user", "".to_string()),
            ("database.password", "".to_string()),
        ]
        .iter()
        .cloned()
        .collect::<HashMap<&'static str, String>>()
    })
}
