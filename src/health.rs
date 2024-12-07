use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Health {
    pub status: HealthStatus,
    pub details: HealthDetails,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Up,
    Down,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthDetails {
    pub chromium: ModuleHealth,
    pub libreoffice: ModuleHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleHealth {
    /// Up / Down Status
    pub status: HealthStatus,

    /// ISO 8601 timestamp
    pub timestamp: String,

    /// If status is `Down`, this field will contain the error message
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub error: Option<String>,
}
