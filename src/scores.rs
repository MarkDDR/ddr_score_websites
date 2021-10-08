use std::collections::HashMap;

use crate::{Client, score_websites::sanbai::get_sanbai_scores};
use anyhow::Result;

use crate::{
    ddr_song::{DDRSong, SongId},
    score_websites::{
        sanbai::SanbaiScoreEntry,
        skill_attack::{SkillAttackIndex, SkillAttackScores},
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Scores {
    pub beg_score: Option<ScoreCombo>,
    pub basic_score: Option<ScoreCombo>,
    pub diff_score: Option<ScoreCombo>,
    pub expert_score: Option<ScoreCombo>,
    pub chal_score: Option<ScoreCombo>,
}

impl Scores {
    pub fn get_score_by_index(&self, index: usize) -> Option<u32> {
        match index {
            0 => self.beg_score.map(|s| s.score),
            1 => self.basic_score.map(|s| s.score),
            2 => self.diff_score.map(|s| s.score),
            3 => self.expert_score.map(|s| s.score),
            4 => self.chal_score.map(|s| s.score),
            _ => None,
        }
    }

    pub fn get_score_by_index_mut(&mut self, index: usize) -> Option<&mut Option<ScoreCombo>> {
        Some(match index {
            0 => &mut self.beg_score,
            1 => &mut self.beg_score,
            2 => &mut self.beg_score,
            3 => &mut self.beg_score,
            4 => &mut self.beg_score,
            _ => return None,
        })
    }

    fn update_from_sanbai_score_entry(&mut self, sanbai_entry: &SanbaiScoreEntry) {
        let score_combo = self.get_score_by_index_mut(sanbai_entry.difficulty as usize).unwrap();
        todo!()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ScoreCombo {
    pub score: u32,
    pub combo: ComboType,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum ComboType {
    /// Skill attack doesn't differeniate between fail and no combo
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

impl ComboType {
    pub fn from_skill_attack_index(index: u8) -> Option<Self> {
        Some(match index {
            0 => Self::FailOrPass,
            1 => Self::GoodGreatCombo,
            2 => Self::PerfectCombo,
            3 => Self::MarvelousCombo,
            _ => return None,
        })
    }

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

    /// Downloads and updates scores by grabbing data from Skill Attack.
    /// Ignores any potentially new song in the Skill Attack data set that
    /// is not accounted for in the `song_list` set
    // TODO have it return a "signal" that new songs may be unaccounted for,
    // with a list of all skipped songs attached
    pub async fn update_scores_from_skill_attack(
        &mut self,
        http: Client,
        song_list: &[DDRSong],
    ) -> Result<()> {
        todo!();
    }

    /// Downloads and updates scores by grabbing data from Sanbai
    pub async fn update_scores_from_sanbai(&mut self, http: Client) -> Result<()> {
        let sanbai_username = match self.sanbai_username.as_deref() {
            Some(name) => name,
            None => return Err(anyhow::anyhow!("Player doesn't have sanbai username")),
        };
        let sanbai_scores = get_sanbai_scores(http, sanbai_username).await?;
        if sanbai_scores.is_empty() {
            return Ok(());
        }
        
        let mut sanbai_scores_iter = sanbai_scores.into_iter();
        // we know there is at least 1 element
        let first = sanbai_scores_iter.next().unwrap();
        let mut last_song_id = first.song_id;
        let mut combined_score = Scores::default();
        
        for score in sanbai_scores {
            if last_song_id == 
        }

        Ok(())
    }

    /// Updates from skill attack scores
    pub fn update_from_sa_scores(sa_scores: &SkillAttackScores, ddr_songs: &[DDRSong]) -> Self {
        let ddr_song_index: HashMap<SkillAttackIndex, &SongId> = ddr_songs
            .iter()
            .filter_map(|s| match s.skill_attack_index {
                Some(sa_index) => Some((sa_index, &s.song_id)),
                None => None,
            })
            .collect();
        let scores: HashMap<_, _> = sa_scores
            .song_score
            .iter()
            .filter_map(|(sa_idx, score)| {
                match ddr_song_index.get(sa_idx) {
                    Some(&song_id) => Some((song_id.to_owned(), *score)),
                    None => {
                        // old long removed song or new song not in db yet
                        // either way ignore
                        None
                    }
                }
            })
            .collect();

        Self {
            name: sa_scores.username.clone(),
            ddr_code: sa_scores.ddr_code,
            scores,
            sanbai_username: None,
        }
    }

    pub fn from_sanbai_scores(
        sanbai_scores: &[SanbaiScoreEntry],
        display_name: String,
        ddr_code: u32,
    ) -> Self {
        // let mut scores = HashMap::new();
        // let's assume
        todo!()
    }
}
