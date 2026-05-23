use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("invalid privacy value: {0}")]
    InvalidPrivacy(String),

    #[error("invalid locale: {0}")]
    InvalidLocale(String),
}
