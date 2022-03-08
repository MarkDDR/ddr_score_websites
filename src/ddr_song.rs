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
            // TODO change the sanbai struct for those names to be `Vec<String>`, split by '/'
            // Multiple names in sanbai are delimated by '/'. The only
            // song in DDR with '/' in its title atm is "I/O", which doesn't
            // have any alternate names. To account for this though we do this split
            // before we add the "raw song name" to the search names
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
        let sa_normalized = {
            let mut sa_normalized = skill_attack_songs
                .iter()
                .map(|s| (normalize_name(&s.song_name), s))
                .collect::<Vec<_>>();
            sa_normalized.sort_by(|(a, _), (b, _)| a.cmp(b));
            sa_normalized
        };
        let sanbai_normalized = {
            let mut sanbai_normalized = sanbai_songs
                .iter()
                .map(|s| (normalize_name(&s.song_name), s))
                .collect::<Vec<_>>();
            sanbai_normalized.sort_by(|(a, _), (b, _)| a.cmp(b));
            sanbai_normalized
        };

        let mut ddr_songs = vec![];
        let mut sa_index = 0;
        let mut sanbai_index = 0;

        loop {
            let (sa_candidate, sanbai_candidate) = match (
                sa_normalized.get(sa_index),
                sanbai_normalized.get(sanbai_index),
            ) {
                (Some(a), Some(b)) => (a, b),
                (None, Some((_, sanbai_song))) => {
                    info!("Leftover song in sanbai: {}", sanbai_song.song_name);
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(sanbai_song, None));
                    sanbai_index += 1;
                    continue;
                }
                _ => break,
            };

            match sa_candidate.0.cmp(&sanbai_candidate.0) {
                std::cmp::Ordering::Equal => {
                    // match, add to vec
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(
                        sanbai_candidate.1,
                        Some(sa_candidate.1),
                    ));
                    sa_index += 1;
                    sanbai_index += 1;
                }
                std::cmp::Ordering::Greater => {
                    // only in sanbai, add to vec
                    // usually this means it is a newer song not yet added to SA
                    // info!("Only in Sanbai, adding: {}", sanbai_candidate.1.song_name);
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(
                        sanbai_candidate.1,
                        None,
                    ));
                    sanbai_index += 1;
                }
                std::cmp::Ordering::Less => {
                    // only in SA, ignore
                    // songs that are only in skill attack are old songs that have been
                    // gone for a long time
                    // info!(
                    //     "Only in Skill Attack, ignoring: {}",
                    //     sa_candidate.1.song_name
                    // );
                    sa_index += 1;
                }
            }
        }

        ddr_songs
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
            Regex::new(r#""sp-bpm">(?P<first>\d+)(-(?P<second>\d+))?<\/span>"#).unwrap()
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

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Chart {
    GSP,
    BSP,
    DSP,
    ESP,
    CSP,
}

impl Chart {
    pub fn is_challenge(&self) -> bool {
        matches!(self, Chart::CSP)
    }

    pub fn from_index(index: usize) -> Option<Self> {
        Some(match index {
            0 => Self::GSP,
            1 => Self::BSP,
            2 => Self::DSP,
            3 => Self::ESP,
            4 => Self::CSP,
            _ => return None,
        })
    }
}

// Differences between Sanbai and Skill Attack/EAmuse site
// - Space between song name and parenteticals `Possession(EDP Mix)`
// - sometimes SA has full width parenthesis, `!`, `+`
// - a couple of smart quotes (over the "period", dreamin')
// - Qipchāq and Qipchãq
// - … and ...
/// Normalize a song name so that slight irregularties in how the name was spelt are ignored
/// when compared
fn normalize_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '！' => '!',
            '（' => '(',
            '）' => ')',
            '“' | '”' => '"',
            'ã' | 'ā' => 'a',
            '＋' => '+',
            '’' => '\'',
            _ => c,
        })
        .flat_map(|c| {
            if c == '…' {
                std::iter::repeat('.').take(3)
            } else {
                std::iter::repeat(c).take(1)
            }
        })
        .collect()
}
