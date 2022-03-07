use crate::ddr_song::{Chart, DDRSong};
use crate::website_backends::skill_attack::SkillAttackIndex;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub enum SearchQuery<'query> {
    ByTitle {
        song_title: &'query str,
        chart_and_level: ChartAndLevel,
    },
    BySkillAttackIndex {
        sa_index: SkillAttackIndex,
        chart_and_level: ChartAndLevel,
    },
}

impl<'query> SearchQuery<'query> {
    // we can't use `FromStr` because we want to borrow from the input string
    // TODO Do we want to differentiate between there not being enough arguments
    // and not being able to parse the last arguments as being difficulties?
    // TODO use real error type. Put it in error.rs?
    pub fn parse_query(query: &'query str) -> Result<Self, NotEnoughArguments> {
        /// Helper function to cut off the level arguments from the query
        fn cut_string_end<'a>(full: &'a str, end: &'a str) -> &'a str {
            let byte_offset = end.as_ptr() as usize - full.as_ptr() as usize;
            &full[..byte_offset]
        }

        let mut last_two = query.split_whitespace();
        // We don't ever want to parse the leading element as the difficulty, so we do this to skip
        // We can't just `.split_whitespace().skip(1)` because then it won't be a
        // `DoubleEndedIterator`
        // This also ensures that the song title can't be empty
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
                        chart_and_level: last.into(),
                    }),
                    Err(_) => Ok(Self::ByTitle {
                        song_title,
                        chart_and_level: last.into(),
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
                        chart_and_level: (c, l).into(),
                    }),
                    Err(_) => Ok(Self::ByTitle {
                        song_title,
                        chart_and_level: (c, l).into(),
                    }),
                }
            }
            // either neither parsed, or not enough arguments given
            _ => Err(NotEnoughArguments),
        }
    }

    pub fn search<'ddr_song>(
        &self,
        song_list: impl IntoIterator<Item = &'ddr_song DDRSong>,
    ) -> Option<SearchResult<'ddr_song>> {
        match *self {
            SearchQuery::ByTitle {
                song_title,
                chart_and_level,
            } => {
                let (search_challenge, search_level) = match chart_and_level {
                    ChartAndLevel::Level(l) => (None, Some(l)),
                    ChartAndLevel::Chart(c) => (Some(c == Chart::CSP), None),
                    ChartAndLevel::Both(c, l) => (Some(c == Chart::CSP), Some(l)),
                };
                let query = song_title.to_lowercase();
                let mut fuzzy_match_candidate = None;

                for song in song_list
                    .into_iter()
                    .filter(|song| {
                        // challenge filter
                        match search_challenge {
                            Some(true) => song.ratings.has_challenge_chart(),
                            Some(false) => song.ratings.has_non_challenge_charts(),
                            None => true, // no info so can't filter filter
                        }
                    })
                    .filter(|song| {
                        // level filter
                        match search_level {
                            Some(l) => song.ratings.contains_single(l),
                            None => true, // no info so can't filter
                        }
                    })
                {
                    // exact match, return right away
                    if song.search_names.first() == Some(&query) {
                        return SearchResult::new(song, chart_and_level);
                    }
                    // fuzzy match over each name/nickname
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
                fuzzy_match_candidate.and_then(|song| SearchResult::new(song, chart_and_level))
            }
            SearchQuery::BySkillAttackIndex {
                sa_index,
                chart_and_level,
            } => {
                // No need to filter, we are just looking for a specific index
                for song in song_list {
                    if song.skill_attack_index == Some(sa_index) {
                        // sanity check
                        return SearchResult::new(song, chart_and_level);
                    }
                }
                // Couldn't find matching skill attack index
                None
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SearchResult<'ddr_song> {
    pub song: &'ddr_song DDRSong,
    pub chart: Chart,
    pub level: u8,
}

impl<'ddr_song> SearchResult<'ddr_song> {
    fn new(song: &'ddr_song DDRSong, chart_and_level: ChartAndLevel) -> Option<Self> {
        let charts = [Chart::GSP, Chart::BSP, Chart::DSP, Chart::ESP, Chart::CSP];
        let mut iter = song.ratings.single_difficulties().into_iter().zip(charts);
        match chart_and_level {
            ChartAndLevel::Level(level) => iter.find(|(l, _)| *l == level),
            ChartAndLevel::Chart(chart) => iter.find(|(l, c)| *c == chart && *l != 0),
            ChartAndLevel::Both(chart, level) => iter.find(|(l, c)| *c == chart && *l == level),
        }
        .map(|(level, chart)| Self { song, chart, level })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct NotEnoughArguments;

/// Represents the different possibilities of parsing the search query,
/// either having only level information, chart information, or both
#[derive(Debug, Copy, Clone)]
pub enum ChartAndLevel {
    /// The level of the song, from 1-19
    Level(u8),
    /// The chart of the song, like DSP or ESP
    Chart(Chart),
    /// both level and chart
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

/// Internal type to help with parsing
#[derive(Debug, Copy, Clone)]
enum DifficultyOrLevel {
    Difficulty(Chart),
    Level(u8),
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
