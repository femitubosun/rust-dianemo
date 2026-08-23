use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct AppSettings {
    pub discovery_port: u16,
    pub discovery_announce_interval_secs: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            discovery_port: 7766,
            discovery_announce_interval_secs: 10,
        }
    }
}
