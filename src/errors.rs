use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, AppError>;
