use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::fmt;
use tracing::info;

pub async fn get_sanbai_song_data(http: Client) -> Result<Vec<SanbaiSong>> {
    let url = "https://3icecream.com/js/songdata.js";
    info!("Sent Sanbai web request");
    let songdata_js = http.get(url).send().await?.text().await?;
    info!("Got Sanbai web page");
    let songdata_js = songdata_js
        .strip_prefix("var ALL_SONG_DATA=")
        .ok_or(anyhow!("missing `ALL_SONG_DATA` prefix"))?
        .strip_suffix(";")
        .ok_or(anyhow!("missing `;` suffix"))?;

    info!("Sanbai parse start");
    let songdata: Vec<SanbaiSong> = serde_json::from_str(songdata_js)?;
    info!("Sanbai parse end");
    Ok(songdata)
}

fn num_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
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
    pub song_id: String,
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
    pub fn get_jacket_url(&self) -> String {
        let base_url = "https://3icecream.com/img/banners/f/";
        format!("{}{}.jpg", base_url, self.song_id)
    }

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

#[derive(Debug, Copy, Clone, Deserialize)]
pub struct LockTypes(pub [i32; 9]);
