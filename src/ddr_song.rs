use std::iter::FromIterator;
use std::str::FromStr;

use tracing::info;

use crate::sanbai::{DDRVersion, Difficulties, LockTypes, SanbaiSong};
use crate::skill_attack::{SkillAttackIndex, SkillAttackSong};

#[derive(Debug, Clone)]
pub struct PlayerScores {}

#[derive(Debug, Clone)]
pub struct DDRSong {
    pub song_id: String,
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
    // pub scores: Vec<PlayerScores>,
}

impl DDRSong {
    pub fn new_from_sanbai_and_skillattack(
        sanbai: &SanbaiSong,
        skill_attack: Option<&SkillAttackSong>,
    ) -> Self {
        let search_names: Vec<String> = sanbai
            .romanized_name
            .iter()
            .map(|s| s.as_str())
            .chain(sanbai.alternate_name.iter().map(|s| s.as_str()))
            .chain(sanbai.searchable_name.iter().map(|s| s.as_str()))
            // TODO change the sanbai struct for those names to be `Vec<String>`, split by '/'
            // Multiple names in sanbai are delimated by '/'. The only
            // song in DDR with '/' in its title atm is "I/O", which doesn't
            // have any alternate names. To account for this though we do this split
            // before we add the "raw song name" to the search names
            .flat_map(|s| s.split('/'))
            .chain(std::iter::once(sanbai.song_name.as_str()))
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
}

pub fn parse_search_query<'query, 'ddr_song>(
    song_list: impl IntoIterator<Item = &'ddr_song DDRSong> + 'ddr_song,
    query: &'query str,
) -> Option<(
    SearchInfo<'query>,
    Box<dyn Iterator<Item = &'ddr_song DDRSong> + 'ddr_song>,
)> {
    let last_two_params: LastTwo<&str> = query.split_whitespace().skip(1).collect();
    let filter: Box<dyn Iterator<Item = &'ddr_song DDRSong>>;
    // let search_title: &str = "TODO";
    let search_info;
    match last_two_params {
        LastTwo::None => return None,
        LastTwo::One(one) => match one.parse::<DifficultyOrLevel>() {
            Ok(dol) => {
                let search_title_offset = one.as_ptr() as usize - query.as_ptr() as usize;
                let search_title = &query[..search_title_offset];
                filter = dol.apply_filter(song_list);
                search_info = SearchInfo::new_title(search_title, dol, None);
            }
            Err(_) => return None,
        },
        LastTwo::Two(one, two) => {
            match (
                one.parse::<DifficultyOrLevel>(),
                two.parse::<DifficultyOrLevel>(),
            ) {
                (
                    Ok(one_dol @ DifficultyOrLevel::Difficulty(_)),
                    Ok(two_dol @ DifficultyOrLevel::Level(_)),
                )
                | (
                    Ok(one_dol @ DifficultyOrLevel::Level(_)),
                    Ok(two_dol @ DifficultyOrLevel::Difficulty(_)),
                ) => {
                    // filter by chart and level
                    let search_title_offset = one.as_ptr() as usize - query.as_ptr() as usize;
                    let search_title = &query[..search_title_offset];
                    let one_filter = one_dol.apply_filter(song_list);
                    filter = two_dol.apply_filter(one_filter);
                    search_info = SearchInfo::new_title(search_title, one_dol, Some(two_dol))
                }
                (
                    Ok(DifficultyOrLevel::Difficulty(_)),
                    Ok(dol @ DifficultyOrLevel::Difficulty(_)),
                )
                | (Err(_), Ok(dol @ DifficultyOrLevel::Difficulty(_)))
                | (Ok(DifficultyOrLevel::Level(_)), Ok(dol @ DifficultyOrLevel::Level(_)))
                | (Err(_), Ok(dol @ DifficultyOrLevel::Level(_))) => {
                    // ignore one, filter two
                    let search_title_offset = two.as_ptr() as usize - query.as_ptr() as usize;
                    let search_title = &query[..search_title_offset];
                    filter = dol.apply_filter(song_list);
                    search_info = SearchInfo::new_title(search_title, dol, None);
                }
                _ => return None,
            }
        }
    }
    Some((search_info, filter))
}

#[derive(Debug, Copy, Clone)]
pub struct SearchInfo<'a> {
    pub search_title: &'a str,
    pub chart: Option<Chart>,
    pub level: Option<u8>,
}

impl<'a> SearchInfo<'a> {
    fn new_title(
        search_title: &'a str,
        one: DifficultyOrLevel,
        two: Option<DifficultyOrLevel>,
    ) -> Self {
        let mut chart = None;
        let mut level = None;
        match one {
            DifficultyOrLevel::Difficulty(c) => chart = Some(c),
            DifficultyOrLevel::Level(l) => level = Some(l),
        }
        match two {
            Some(DifficultyOrLevel::Difficulty(c)) => chart = Some(c),
            Some(DifficultyOrLevel::Level(l)) => level = Some(l),
            None => {}
        }

        Self {
            search_title: search_title.trim(),
            chart,
            level,
        }
    }

    pub fn search_title(&self) -> &'a str {
        match self {
            Self { search_title, .. } => search_title,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub enum DifficultyOrLevel {
    Difficulty(Chart),
    Level(u8),
}

impl DifficultyOrLevel {
    fn apply_filter<'a>(
        self,
        song_list: impl IntoIterator<Item = &'a DDRSong> + 'a,
    ) -> Box<dyn Iterator<Item = &'a DDRSong> + 'a> {
        match self {
            DifficultyOrLevel::Difficulty(Chart::CSP) => {
                Box::new(filter_by_has_challenge(song_list))
            }
            // _ => todo!(),
            DifficultyOrLevel::Difficulty(_) => Box::new(filter_by_has_non_challenge(song_list)),
            DifficultyOrLevel::Level(level) => {
                Box::new(filter_by_single_difficulty(song_list, level))
            }
        }
    }
}

impl FromStr for DifficultyOrLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Chart::*;
        use DifficultyOrLevel::*;
        match s {
            "gsp" => Ok(Difficulty(GSP)),
            "bsp" => Ok(Difficulty(BSP)),
            "dsp" => Ok(Difficulty(DSP)),
            "esp" => Ok(Difficulty(ESP)),
            "csp" => Ok(Difficulty(CSP)),
            _ => {
                if let Ok(level) = s.parse::<u8>() {
                    if level < 20 {
                        return Ok(Level(level));
                    }
                }
                Err(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LastTwo<T> {
    None,
    One(T),
    Two(T, T),
}

impl<T> FromIterator<T> for LastTwo<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let (mut a, mut b) = (None, None);
        while let Some(x) = iter.next() {
            a = b;
            b = Some(x);
        }
        match (a, b) {
            (None, None) => Self::None,
            (None, Some(one)) => Self::One(one),
            (Some(one), Some(two)) => Self::Two(one, two),
            (Some(_), None) => unreachable!(),
        }
    }
}

/// Perform a fuzzy search on the song list based on song title
pub fn search_by_title<'a>(
    song_list: impl IntoIterator<Item = &'a DDRSong>,
    query: &str,
) -> Option<&'a DDRSong> {
    if query.is_empty() {
        return None;
    }
    let query = query.to_lowercase();
    let mut fuzzy_match_candidate: Option<&DDRSong> = None;

    for song in song_list {
        // exact match, return right away
        if song.search_names.last() == Some(&query) {
            return Some(song);
        }
        // fuzzy match
        'next_name: for search_name in &song.search_names {
            for query_word in query.split_whitespace() {
                if !search_name.contains(query_word) {
                    continue 'next_name;
                }
            }
            // we can try to employ some better heuristics here
            // current: Use first one alphabetically
            if fuzzy_match_candidate.is_none() {
                fuzzy_match_candidate = Some(song);
            }
        }
    }
    fuzzy_match_candidate
}

/// Search the song list based on a skill attack ID
pub fn search_by_skill_attack_id<'a>(
    song_list: impl IntoIterator<Item = &'a DDRSong>,
    query: SkillAttackIndex,
) -> Option<&'a DDRSong> {
    for song in song_list {
        if song.skill_attack_index == Some(query) {
            return Some(song);
        }
    }
    None
}

/// Filters songs based on if they contain a specific difficulty
pub fn filter_by_single_difficulty<'a, I>(
    song_list: I,
    difficulty: u8,
) -> impl Iterator<Item = &'a DDRSong>
where
    I: IntoIterator<Item = &'a DDRSong>,
{
    song_list
        .into_iter()
        .filter(move |s| s.ratings.contains_single(difficulty))
}

/// Filters away any song that lacks a challenge chart
pub fn filter_by_has_challenge<'a, I>(song_list: I) -> impl Iterator<Item = &'a DDRSong>
where
    I: IntoIterator<Item = &'a DDRSong>,
{
    song_list
        .into_iter()
        .filter(|s| s.ratings.has_single_challenge())
}

/// Filters away any song that lacks a non-challenge chart
pub fn filter_by_has_non_challenge<'a, I>(song_list: I) -> impl Iterator<Item = &'a DDRSong>
where
    I: IntoIterator<Item = &'a DDRSong>,
{
    song_list
        .into_iter()
        .filter(|s| s.ratings.has_non_challenge())
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
