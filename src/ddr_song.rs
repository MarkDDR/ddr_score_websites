use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use tracing::{info, warn};

use crate::website_backends::sanbai::{DDRVersion, Difficulties, LockTypes, SanbaiSong};
use crate::website_backends::skill_attack::{SkillAttackIndex, SkillAttackSong};
use crate::{HttpClient, Result};

mod song_id;
pub use song_id::SongId;

#[derive(Debug, Clone)]
pub struct DDRSong {
    pub song_id: SongId,
    pub skill_attack_index: Option<SkillAttackIndex>,
    pub song_name: String,
    pub romanized_name: Option<String>,
    /// A list of all variations of the song name, all lowercase
    pub search_names: Vec<String>,
    pub version_num: DDRVersion,
    pub deleted: bool,
    pub ratings: Difficulties,
    // Lock condition, i.e. Extra Savior, Golden League, Unlock Event, etc.
    pub lock_types: Option<LockTypes>,
}

impl DDRSong {
    pub fn new_from_sanbai_and_skillattack(
        sanbai: &SanbaiSong,
        skill_attack: Option<&SkillAttackSong>,
    ) -> Self {
        let search_names: Vec<String> = std::iter::once(sanbai.song_name.as_str())
            .chain(sanbai.romanized_name.as_deref())
            .chain(sanbai.alternate_name.iter().flat_map(|s| s.split('/')))
            .chain(sanbai.searchable_name.iter().flat_map(|s| s.split('/')))
            .map(|s| s.to_lowercase())
            .collect();
        Self {
            song_id: sanbai.song_id.clone(),
            skill_attack_index: skill_attack.map(|s| s.skill_attack_index),
            song_name: sanbai.song_name.clone(),
            romanized_name: sanbai.romanized_name.clone(),
            search_names,
            version_num: sanbai.version_num,
            deleted: sanbai.deleted,
            ratings: sanbai.ratings,
            lock_types: sanbai.lock_types,
        }
    }

    pub fn from_combining_song_lists(
        sanbai_songs: &[SanbaiSong],
        skill_attack_songs: &[SkillAttackSong],
    ) -> Vec<Self> {
        info!("Combining sanbai and skill attack song lists");
        let mut ddr_song_map: HashMap<SongId, Self> = sanbai_songs
            .iter()
            .map(|s| {
                (
                    s.song_id.clone(),
                    Self::new_from_sanbai_and_skillattack(&s, None),
                )
            })
            .collect();

        for sa_song in skill_attack_songs {
            // If we don't find a corresponding song in the map, that means that
            // it is usually an old skill attack song that hasn't been in the game
            // for many many years
            // We are ignoring the possibility that Skill Attack somehow updated its
            // song list before Sanbai, as Sanbai is generally much faster and more
            // on top of its song list. Sanbai also usually has more information about the
            // song so we consider it more valuable than only having skill attack info
            if let Some(ddr_song) = ddr_song_map.get_mut(&sa_song.song_id) {
                // TODO sanity check on difficulties? If only to emit a warning in the logs
                // TODO sanity check on song name? We already know that Sanbai changed some of
                // the names slightly at first in attempt to make searching easier, like
                // by changing some full width characters to half width, some smart quotes, etc.
                ddr_song.skill_attack_index = Some(sa_song.skill_attack_index);
            }
        }

        let mut out: Vec<_> = ddr_song_map.into_values().collect();
        // Sort for consistency
        out.sort_by(|a, b| a.song_name.cmp(&b.song_name));
        info!("Combining complete");
        out
    }

    pub async fn fetch_bpm(&self, http: HttpClient) -> Result<Option<Bpm>> {
        // Matches strings like this
        // "<span class="sp-bpm">75-528</span>"
        //              ^--------++-+++------^
        //                       ^^ ^^^
        //                       |     \
        //                       first  second
        // "<span class="sp-bpm">150</span>"
        //              ^--------+++------^
        //                       ^^^
        //                       |
        //                       first
        static SP_BPM_FINDER: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#""sp-bpm">(?P<first>\d+)(-(?P<second>\d+))?</span>"#).unwrap()
        });

        let song_info_url = format!("https://3icecream.com/ddr/song_details/{}", self.song_id);

        let response = http.get(song_info_url).send().await?.text().await?;
        let mut cap_iter = SP_BPM_FINDER.captures_iter(&response);
        if let Some(cap) = cap_iter.next() {
            match (cap.name("first"), cap.name("second")) {
                (Some(first_cap), Some(second_cap)) => {
                    let lower = first_cap.as_str().parse::<u16>().expect("Really big bpm");
                    let upper = second_cap.as_str().parse::<u16>().expect("Really big bpm");
                    if let Some(main_bpm_cap) = cap_iter.next() {
                        let main = main_bpm_cap
                            .name("first")
                            .expect("This should be impossible")
                            .as_str()
                            .parse::<u16>()
                            .expect("Really big bpm");
                        Ok(Some(Bpm::Range { lower, upper, main }))
                    } else {
                        warn!("We couldn't find the main bpm!");
                        Err(crate::error::Error::SanbaiBpmHtmlParseError)
                    }
                }
                (Some(first_cap), None) => {
                    let bpm = first_cap.as_str().parse::<u16>().expect("Really big bpm");
                    Ok(Some(Bpm::Constant(bpm)))
                }
                _ => unreachable!("This case should be impossible"),
            }
        } else {
            // Sanity check, we should see a `"sp-missing-bpm"` in the html
            // if not something may have changed with the html so we should give an error for that
            if response.contains(r#""sp-missing-bpm""#) {
                Ok(None)
            } else {
                warn!("Bpm html might have changed!");
                Err(crate::error::Error::SanbaiBpmHtmlParseError)
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Bpm {
    Constant(u16),
    Range { lower: u16, upper: u16, main: u16 },
}

impl Bpm {
    pub fn get_main_bpm(&self) -> u16 {
        match *self {
            Bpm::Constant(m) => m,
            Bpm::Range { main, .. } => main,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Chart {
    GSP,
    BSP,
    DSP,
    ESP,
    CSP,
    BDP,
    DDP,
    EDP,
    CDP,
}

impl Chart {
    pub fn is_challenge(&self) -> bool {
        matches!(self, Chart::CSP)
    }

    pub fn is_doubles(&self) -> bool {
        *self as u8 > 4
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => Self::GSP,
            1 => Self::BSP,
            2 => Self::DSP,
            3 => Self::ESP,
            4 => Self::CSP,
            5 => Self::BDP,
            6 => Self::DDP,
            7 => Self::EDP,
            8 => Self::CDP,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Chart;
    #[test]
    fn chart_is_doubles() {
        assert!(!Chart::GSP.is_doubles());
        assert!(!Chart::BSP.is_doubles());
        assert!(!Chart::DSP.is_doubles());
        assert!(!Chart::ESP.is_doubles());
        assert!(!Chart::CSP.is_doubles());
        assert!(Chart::BDP.is_doubles());
        assert!(Chart::DDP.is_doubles());
        assert!(Chart::EDP.is_doubles());
        assert!(Chart::CDP.is_doubles());
    }
}
// Differences between Sanbai and Skill Attack/EAmuse site
// - Space between song name and parenteticals `Possession(EDP Mix)`
// - sometimes SA has full width parenthesis, `!`, `+`
// - a couple of smart quotes (over the "period", dreamin')
// - Qipchāq and Qipchãq
// - … and ...
// /// Normalize a song name so that slight irregularties in how the name was spelt are ignored
// /// when compared
// fn normalize_name(input: &str) -> String {
//     input
//         .chars()
//         .filter(|c| !c.is_whitespace())
//         .map(|c| match c {
//             '！' => '!',
//             '（' => '(',
//             '）' => ')',
//             '“' | '”' => '"',
//             'ã' | 'ā' => 'a',
//             '＋' => '+',
//             '’' => '\'',
//             _ => c,
//         })
//         .flat_map(|c| {
//             if c == '…' {
//                 std::iter::repeat('.').take(3)
//             } else {
//                 std::iter::repeat(c).take(1)
//             }
//         })
//         .collect()
// }
