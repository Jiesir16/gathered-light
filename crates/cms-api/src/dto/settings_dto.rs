use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingsResp {
    pub theme: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateThemeReq {
    #[validate(custom(function = "validate_theme"))]
    pub theme: String,
}

pub fn validate_theme(value: &str) -> Result<(), ValidationError> {
    if ["warm", "cool", "bold"].contains(&value) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid theme"))
    }
}
