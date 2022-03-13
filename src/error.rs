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
    #[error("Couldn't parse the bpm html, something may have changed")]
    SanbaiBpmHtmlParseError,
    #[error("Couldn't parse skill attack html, something may have changed")]
    SkillAttackHtmlParseError(&'static str),
}
