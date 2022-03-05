use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use crate::{ddr_song::SongId, score_websites::sanbai::SanbaiScoreEntry};

/// The scores and lamp for every difficulty of a specific song
#[derive(Debug, Clone, Copy, Default)]
pub struct Scores {
    pub beg_score: Option<ScoreRow>,
    pub basic_score: Option<ScoreRow>,
    pub diff_score: Option<ScoreRow>,
    pub expert_score: Option<ScoreRow>,
    pub chal_score: Option<ScoreRow>,
}

impl Scores {
    /// Updates the score by comparing the scores in other and taking the
    /// score and lamp type of both
    pub fn update(&mut self, other: &Self) {
        for level_index in 0..=4 {
            let new_score = match (self[level_index], other[level_index]) {
                (Some(our_score), Some(other_score)) => Some(our_score.maximize(other_score)),
                (None, Some(only_score)) | (Some(only_score), None) => Some(only_score),
                (None, None) => None,
            };
            self[level_index] = new_score;
        }
    }

    /// Updates the score and lamp type of a single difficulty specified by
    /// sanbai entry, taking the max
    pub fn update_from_sanbai_score_entry(&mut self, sanbai_entry: &SanbaiScoreEntry) {
        // FIXME we are ignoring doubles scores for now
        if sanbai_entry.difficulty > 4 {
            return;
        }
        let score_combo = &mut self[sanbai_entry.difficulty as usize];

        match score_combo.as_mut() {
            Some(difficulty) => {
                difficulty.score = std::cmp::max(difficulty.score, sanbai_entry.score);
                difficulty.lamp = std::cmp::max(difficulty.lamp, sanbai_entry.lamp);
            }
            None => {
                *score_combo = Some(ScoreRow {
                    score: sanbai_entry.score,
                    lamp: sanbai_entry.lamp,
                });
            }
        };
    }
}

impl Index<usize> for Scores {
    type Output = Option<ScoreRow>;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.beg_score,
            1 => &self.basic_score,
            2 => &self.diff_score,
            3 => &self.expert_score,
            4 => &self.chal_score,
            _ => panic!("Invalid score index"),
        }
    }
}

impl IndexMut<usize> for Scores {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.beg_score,
            1 => &mut self.basic_score,
            2 => &mut self.diff_score,
            3 => &mut self.expert_score,
            4 => &mut self.chal_score,
            _ => panic!("Invalid score index"),
        }
    }
}

/// A "row" of a score, representing the score and lamp of a specific difficulty of a song
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ScoreRow {
    pub score: u32,
    pub lamp: LampType,
}

impl ScoreRow {
    /// Creates a new `ScoreRow` by comparing `self` and `other` and taking
    /// the max of `score` and the max of `lamp`.
    ///
    /// # Examples
    /// ```rust
    /// use score_websites::scores::{ScoreRow, LampType};
    ///
    /// let score_a = ScoreRow {
    ///     score: 890_000,
    ///     lamp: LampType::GreatCombo,
    /// };
    /// let score_b = ScoreRow {
    ///     score: 950_000,
    ///     lamp: LampType::NoCombo,
    /// };
    /// assert_eq!(score_a.maximize(score_b), ScoreRow {
    ///     score: 950_000,
    ///     lamp: LampType::GreatCombo,
    /// });
    /// ```
    pub fn maximize(self, other: Self) -> Self {
        let mut new = self.clone();
        new.score = std::cmp::max(self.score, other.score);
        new.lamp = std::cmp::max(self.lamp, other.lamp);
        new
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum LampType {
    /// Skill attack doesn't differeniate between fail and pass
    FailOrPass,
    Fail,
    NoCombo,
    Life4Combo,
    /// Skill attack doesn't differeniate between good and great combo
    GoodGreatCombo,
    GoodCombo,
    GreatCombo,
    PerfectCombo,
    MarvelousCombo,
}

impl LampType {
    /// Converts the integer Skill Attack uses to represent their combo type
    /// into `LampType`
    pub fn from_skill_attack_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::FailOrPass,
            1 => Self::GoodGreatCombo,
            2 => Self::PerfectCombo,
            3 => Self::MarvelousCombo,
            _ => return None,
        })
    }

    /// Converts the integer Sanbai uses to represent their combo type
    /// into `LampType`
    pub fn from_sanbai_lamp_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::Fail,
            1 => Self::NoCombo,
            2 => Self::Life4Combo,
            3 => Self::GoodCombo,
            4 => Self::GreatCombo,
            5 => Self::PerfectCombo,
            6 => Self::MarvelousCombo,
            _ => return None,
        })
    }
}

/// Represents a specific DDR player, including their scores.
#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub ddr_code: u32,
    pub sanbai_username: Option<String>,
    pub scores: HashMap<SongId, Scores>,
}

impl Player {
    /// Creates a new player with no scores initalized
    pub fn new(
        display_name: impl Into<String>,
        ddr_code: u32,
        sanbai_username: Option<impl Into<String>>,
    ) -> Self {
        Self {
            name: display_name.into(),
            ddr_code,
            sanbai_username: sanbai_username.map(Into::into),
            scores: HashMap::new(),
        }
    }
}
