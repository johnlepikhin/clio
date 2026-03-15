use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("data integrity error: {0}")]
    DataIntegrity(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

impl From<arboard::Error> for AppError {
    fn from(e: arboard::Error) -> Self {
        AppError::Clipboard(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
