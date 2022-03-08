use crate::error::{Error, Result};
use crate::HttpClient;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::fmt;
use std::result::Result as StdResult;
use tracing::info;

use crate::{ddr_song::SongId, scores::LampType};

pub async fn get_sanbai_song_data(http: HttpClient) -> Result<Vec<SanbaiSong>> {
    let url = "https://3icecream.com/js/songdata.js";
    info!("Sent Sanbai web request");
    let songdata_js = http.get(url).send().await?.text().await?;
    info!("Got Sanbai web page");
    let songdata_js = songdata_js
        .strip_prefix("var ALL_SONG_DATA=")
        .ok_or(Error::OtherParseError("missing `ALL_SONG_DATA` prefix"))?
        .strip_suffix(';')
        .ok_or(Error::OtherParseError("missing `;` suffix"))?;

    info!("Sanbai parse start");
    let songdata: Vec<SanbaiSong> =
        serde_json::from_str(songdata_js).map_err(|e| Error::SanbaiSongJsonParseError(e))?;
    info!("Sanbai parse end");
    Ok(songdata)
}

fn num_to_bool<'de, D>(deserializer: D) -> StdResult<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = <i32>::deserialize(deserializer)?;
    Ok(match num {
        1 => true,
        _ => false,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct SanbaiSong {
    pub song_id: SongId,
    pub song_name: String,
    pub alternate_name: Option<String>,
    pub romanized_name: Option<String>,
    pub searchable_name: Option<String>,
    // alphabet sort
    // pub alphabet: char,
    pub version_num: DDRVersion,
    #[serde(default)]
    #[serde(deserialize_with = "num_to_bool")]
    pub deleted: bool,
    pub ratings: Difficulties,
    // Lock condition, i.e. Extra Savior, Golden League, Unlock Event, etc.
    pub lock_types: Option<LockTypes>,
}

impl SanbaiSong {
    /// 256x256 jacket
    pub fn get_jacket_url(&self) -> String {
        let base_url = "https://3icecream.com/img/banners/f/";
        format!("{}{}.jpg", base_url, self.song_id)
    }

    /// 120x120 jacket
    pub fn get_small_jacket_url(&self) -> String {
        let base_url = "https://3icecream.com/img/banners/";
        format!("{}{}.jpg", base_url, self.song_id)
    }

    pub fn has_sp_level(&self, level: u8) -> bool {
        self.ratings.0[0..5].iter().any(|&x| x == level)
    }

    pub fn has_dp_level(&self, level: u8) -> bool {
        self.ratings.0[5..].iter().any(|&x| x == level)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize_repr)]
#[repr(u8)]
pub enum DDRVersion {
    #[serde(other)]
    UnknownVersion,
    DDRA20Plus = 18,
    DDRA20 = 17,
    DDRA = 16,
    DDR2014 = 15,
    DDR2013 = 14,
    DDRX3 = 13,
    DDRX2 = 12,
    DDRX = 11,
    DDRSuperNOVA2 = 10,
    DDRSuperNOVA = 9,
    DDREXTREME = 8,
    DDRMAX2 = 7,
    DDRMAX = 6,
    DDR5thMIX = 5,
    DDR4thMIX = 4,
    DDR3rdMIX = 3,
    DDR2ndMIX = 2,
    DDR1stMIX = 1,
}

impl fmt::Display for DDRVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            DDRVersion::DDRA20Plus => write!(f, "Dance Dance Revolution A20 PLUS"),
            DDRVersion::DDRA20 => write!(f, "Dance Dance Revolution A20"),
            DDRVersion::DDRA => write!(f, "Dance Dance Revolution A"),
            DDRVersion::DDR2014 => write!(f, "Dance Dance Revolution 2014"),
            DDRVersion::DDR2013 => write!(f, "Dance Dance Revolution 2013"),
            DDRVersion::DDRX3 => write!(f, "Dance Dance Revolution X3 VS 2ndMIX"),
            DDRVersion::DDRX2 => write!(f, "Dance Dance Revolution X2"),
            DDRVersion::DDRX => write!(f, "Dance Dance Revolution X"),
            DDRVersion::DDRSuperNOVA2 => write!(f, "Dance Dance Revolution SuperNOVA2"),
            DDRVersion::DDRSuperNOVA => write!(f, "Dance Dance Revolution SuperNOVA"),
            DDRVersion::DDREXTREME => write!(f, "Dance Dance Revolution EXTREME"),
            DDRVersion::DDRMAX2 => write!(f, "Dance Dance Revolution MAX2"),
            DDRVersion::DDRMAX => write!(f, "Dance Dance Revolution MAX"),
            DDRVersion::DDR5thMIX => write!(f, "Dance Dance Revolution 5thMIX"),
            DDRVersion::DDR4thMIX => write!(f, "Dance Dance Revolution 4thMIX"),
            DDRVersion::DDR3rdMIX => write!(f, "Dance Dance Revolution 3rdMIX"),
            DDRVersion::DDR2ndMIX => write!(f, "Dance Dance Revolution 2ndMIX"),
            DDRVersion::DDR1stMIX => write!(f, "Dance Dance Revolution 1stMIX"),
            DDRVersion::UnknownVersion => write!(f, "Unknown Version"),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct Difficulties(pub [u8; 9]);

impl Difficulties {
    pub fn contains_single(&self, difficulty: u8) -> bool {
        self.0[0..5].contains(&difficulty)
    }

    /// Returns true if the song has a CSP or CDP chart
    pub fn has_challenge_chart(&self) -> bool {
        self.0[4] > 0
    }

    /// Returns true if the song has charts that aren't CSP
    pub fn has_non_challenge_charts(&self) -> bool {
        self.0[0] > 0
    }

    pub fn single_difficulties(&self) -> [u8; 5] {
        self.0[0..5].try_into().unwrap()
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct LockTypes(pub [i32; 9]);

// Sanbai scores
// we can get the scores of a user by sending a POST request to
// https://3icecream.com/api/follow_scores
// with the following json
// {
//   //SP_or_DP: 0
//   username: "sanbai_username"
// }
// and it returns the following json object
// { "scores": [
//   {
//      "song_id": "0088dOQPiD0Qb0Dl8ol09D98IOllI1id",
//      "SP_or_DP": 0,
//      "difficulty": 2,
//      "score": 989350,
//      "prev_score": 983570,
//      "lamp": 4,
//      "time_played": 1620500291,
//      "time_scraped": 1620522577
//   },
//   { /* etc. */ }
// ]}
// different difficulties of the same song are usually placed next to each other
// in the json array, so keep that in mind when updating/inserting scores into a user's
// score hashmap

// lamps:
// 0 = fail
// 1 = clear
// 2 = Unknown, but presumably life4
// 3 = goodFC
// 4 = greatFC
// 5 = PFC
// 6 = MFC
#[derive(Debug, Clone, Deserialize)]
pub struct SanbaiScoreEntry {
    pub song_id: SongId,
    pub difficulty: u8,
    pub score: u32,
    #[serde(deserialize_with = "num_to_sanbai_combo")]
    pub lamp: LampType,
}

fn num_to_sanbai_combo<'de, D>(deserializer: D) -> StdResult<LampType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = <u8>::deserialize(deserializer)?;
    Ok(match LampType::from_sanbai_lamp_index(num) {
        Some(c) => c,
        None => todo!("Add unrecognized number error"),
    })
}

#[derive(Debug, Deserialize)]
struct SanbaiScoreOuter {
    scores: Vec<SanbaiScoreEntry>,
}

pub async fn get_sanbai_scores(http: HttpClient, username: &str) -> Result<Vec<SanbaiScoreEntry>> {
    let url = "https://3icecream.com/api/follow_scores";
    let json_data = serde_json::json!({
        "username": username,
    });

    info!("Sent for Sanbai scores");
    let scores_outer = http
        .post(url)
        .json(&json_data)
        .send()
        .await?
        .json::<SanbaiScoreOuter>()
        .await;
    let scores_outer = match scores_outer {
        Ok(x) => x,
        Err(e) if e.is_decode() => return Err(Error::SanbaiScoreJsonParseError(e)),
        Err(e) => return Err(e.into()),
    };
    info!("Received sanbai scores");
    Ok(scores_outer.scores)
}
