use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingsResp {
    pub theme: String,
    #[serde(default = "default_range")]
    pub range: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThemeReq {
    #[validate(custom(function = "validate_theme"))]
    pub theme: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRangeReq {
    #[validate(custom(function = "validate_range"))]
    pub range: String,
}

pub fn validate_theme(value: &str) -> Result<(), ValidationError> {
    if ["warm", "cool", "bold"].contains(&value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid theme"))
    }
}

pub fn validate_range(value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if (4..=32).contains(&trimmed.chars().count())
        && !trimmed.contains('\n')
        && !trimmed.contains('\r')
    {
        Ok(())
    } else {
        Err(ValidationError::new("invalid range"))
    }
}

fn default_range() -> String {
    "2025 - 2026".to_owned()
}
