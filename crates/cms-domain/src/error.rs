use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("invalid privacy value: {0}")]
    InvalidPrivacy(String),

    #[error("invalid locale: {0}")]
    InvalidLocale(String),

    #[error("invalid post type: {0}")]
    InvalidPostType(String),

    #[error("invalid post status: {0}")]
    InvalidPostStatus(String),
}
