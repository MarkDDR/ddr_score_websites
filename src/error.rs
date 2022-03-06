pub type Result<T> = std::result::Result<T, Error>;

use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum Error {
    #[error("Http Error")]
    HttpError(#[from] reqwest::Error),
    // #[error("Error parsing json response")]
    // JsonError(#[from] serde_json::Error),
    #[error("Error parsing response")]
    OtherParseError(&'static str),
    #[error("Error parsing into `Vec<SanbaiSong>`")]
    SanbaiSongJsonParseError(serde_json::Error),
    #[error("Error parsing into `SanbaiScoreOuter`")]
    SanbaiScoreJsonParseError(reqwest::Error),
}
