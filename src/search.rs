use crate::ddr_song::{Chart, DDRSong};
use crate::score_websites::skill_attack::SkillAttackIndex;
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy)]
pub enum SearchQuery<'query> {
    ByTitle {
        song_title: &'query str,
        difficulty: ChartAndLevel,
    },
    BySkillAttackIndex {
        sa_index: SkillAttackIndex,
        difficulty: ChartAndLevel,
    },
}

impl<'query> SearchQuery<'query> {
    // we can't use `FromStr` because we want to borrow from the input string
    // TODO Do we want to differentiate between there not being enough arguments
    // and not being able to parse the last arguments as being difficulties?
    // TODO use real error type. Put it in error.rs?
    pub fn parse_query(query: &'query str) -> Result<Self, ()> {
        let mut last_two = query.split_whitespace();
        // We don't ever want to parse the leading element, so we do this to skip
        // We can't just `.split_whitespace().skip(1)` because then it won't be a
        // `DoubleEndedIterator`
        last_two.next();
        let last_str = last_two.next_back();
        let last = last_str
            .map(|s| s.parse::<DifficultyOrLevel>().ok())
            .flatten();
        let penultimate_str = last_two.next_back();
        let penultimate = penultimate_str
            .map(|s| s.parse::<DifficultyOrLevel>().ok())
            .flatten();
        use DifficultyOrLevel::*;
        match (last, penultimate) {
            // Either only last parsed, or last and penultimate parsed to the same variant type
            // ignore penultimate and just use last
            (Some(last), None)
            | (Some(last @ Difficulty(_)), Some(Difficulty(_)))
            | (Some(last @ Level(_)), Some(Level(_))) => {
                let song_title = cut_string_end(query, last_str.unwrap()).trim();
                match song_title.parse::<SkillAttackIndex>() {
                    Ok(sa_index) => Ok(Self::BySkillAttackIndex {
                        sa_index,
                        difficulty: last.into(),
                    }),
                    Err(_) => Ok(Self::ByTitle {
                        song_title,
                        difficulty: last.into(),
                    }),
                }
            }
            // last and penultimate parsed to different variants, incorporate both and cut
            // at where penultimate started
            (Some(Difficulty(c)), Some(Level(l))) | (Some(Level(l)), Some(Difficulty(c))) => {
                let song_title = cut_string_end(query, penultimate_str.unwrap()).trim();
                match song_title.parse::<SkillAttackIndex>() {
                    Ok(sa_index) => Ok(Self::BySkillAttackIndex {
                        sa_index,
                        difficulty: (c, l).into(),
                    }),
                    Err(_) => Ok(Self::ByTitle {
                        song_title,
                        difficulty: (c, l).into(),
                    }),
                }
            }
            // either neither parsed, or not enough arguments given
            _ => todo!("return Please input difficulty error"),
        }
    }
}

fn cut_string_end<'a>(full: &'a str, end: &'a str) -> &'a str {
    let byte_offset = end.as_ptr() as usize - full.as_ptr() as usize;
    &full[..byte_offset]
}

#[derive(Debug, Copy, Clone)]
pub enum ChartAndLevel {
    Level(u8),
    Chart(Chart),
    Both(Chart, u8),
}

impl From<DifficultyOrLevel> for ChartAndLevel {
    fn from(d_or_l: DifficultyOrLevel) -> Self {
        match d_or_l {
            DifficultyOrLevel::Difficulty(c) => Self::Chart(c),
            DifficultyOrLevel::Level(l) => Self::Level(l),
        }
    }
}

impl From<(Chart, u8)> for ChartAndLevel {
    fn from((c, l): (Chart, u8)) -> Self {
        Self::Both(c, l)
    }
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

#[derive(Debug, Copy, Clone)]
enum DifficultyOrLevel {
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
            "gsp" | "GSP" | "Gsp" | "bSP" => Ok(Difficulty(GSP)),
            "bsp" | "BSP" | "Bsp" => Ok(Difficulty(BSP)),
            "dsp" | "DSP" | "Dsp" => Ok(Difficulty(DSP)),
            "esp" | "ESP" | "Esp" => Ok(Difficulty(ESP)),
            "csp" | "CSP" | "Csp" => Ok(Difficulty(CSP)),
            _ => {
                if let Ok(level) = s.parse::<u8>() {
                    if 0 < level && level < 20 {
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
