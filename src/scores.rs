use std::{
    collections::HashMap,
    future::Future,
    ops::{Index, IndexMut},
    sync::Arc,
};

use crate::{
    score_websites::{
        sanbai::get_sanbai_scores,
        skill_attack::{self, SkillAttackSong},
    },
    Client,
};
use anyhow::Result;
use tokio::sync::oneshot;

use crate::{
    ddr_song::{DDRSong, SongId},
    score_websites::{sanbai::SanbaiScoreEntry, skill_attack::SkillAttackIndex},
};

/// The scores and lamp colors of each difficulty of a specific song
#[derive(Debug, Clone, Copy, Default)]
pub struct Scores {
    pub beg_score: Option<ScoreCombo>,
    pub basic_score: Option<ScoreCombo>,
    pub diff_score: Option<ScoreCombo>,
    pub expert_score: Option<ScoreCombo>,
    pub chal_score: Option<ScoreCombo>,
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
        let score_combo = &mut self[sanbai_entry.difficulty as usize];

        match score_combo.as_mut() {
            Some(difficulty) => {
                difficulty.score = std::cmp::max(difficulty.score, sanbai_entry.score);
                difficulty.lamp = std::cmp::max(difficulty.lamp, sanbai_entry.lamp);
            }
            None => {
                *score_combo = Some(ScoreCombo {
                    score: sanbai_entry.score,
                    lamp: sanbai_entry.lamp,
                });
            }
        };
    }
}

impl Index<usize> for Scores {
    type Output = Option<ScoreCombo>;

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

/// The score and lamp color for a single unspecified difficulty of an unspecified song
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ScoreCombo {
    pub score: u32,
    pub lamp: LampType,
}

impl ScoreCombo {
    /// Creates a new `ScoreCombo` by comparing `self` and `other` and taking
    /// the max of `score` and the max of `lamp`.
    ///
    /// # Examples
    /// ```rust
    /// use score_websites::scores::{ScoreCombo, LampType};
    ///
    /// let score_a = ScoreCombo {
    ///     score: 890_000,
    ///     lamp: LampType::GreatCombo,
    /// };
    /// let score_b = ScoreCombo {
    ///     score: 950_000,
    ///     lamp: LampType::NoCombo,
    /// };
    /// assert_eq!(score_a.maximize(score_b), ScoreCombo {
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

    /// Downloads and updates scores by grabbing data from Skill Attack.
    ///
    /// # `song_list` Future
    /// `song_list` is needed to convert the raw Skill Attack scores into the format we use
    /// and needs data from Skill Attack to be updated, but querying Skill Attack
    /// is very slow and we don't want to block grabbing the raw Skill Attack scores
    /// for every player while we wait for one request from Skill Attack so we can update the
    /// `song_list`, so instead we use a `Future` so we don't block grabbing the raw scores.
    /// The idea is the `Future` can be the receiving end of some kind of
    /// "Single Producer, Multiple Consumer"
    pub async fn update_scores_from_skill_attack(
        &mut self,
        http: Client,
        song_list: impl Future<Output = Arc<Vec<DDRSong>>>,
    ) -> Result<()> {
        let sa_scores = skill_attack::get_scores(http, self.ddr_code).await?;
        let song_list = song_list.await;

        self.update_skill_attack_inner(song_list, sa_scores);

        Ok(())
    }

    /// Downloads scores and song list from Skill Attack. Sends the raw Skill Attack
    /// songs back through the oneshot sender to be processed into our own
    /// song list format.
    ///
    /// # Why is updating the scores and songs combined here?
    /// We can scrap a single web page on Skill Attack for both song and score data. Unfortunately
    /// Skill Attack is very slow, so we want to avoid making duplicate requests if we can.
    pub async fn update_scores_and_songs_from_skill_attack(
        &mut self,
        http: Client,
        sa_song_sender: oneshot::Sender<Vec<SkillAttackSong>>,
        song_list: impl Future<Output = Arc<Vec<DDRSong>>>,
    ) -> Result<()> {
        let (sa_scores, sa_songs) =
            skill_attack::get_scores_and_song_data(http, self.ddr_code).await?;
        sa_song_sender
            .send(sa_songs)
            .map_err(|_| anyhow::anyhow!("oneshot closed early"))?;
        let song_list = song_list.await;

        self.update_skill_attack_inner(song_list, sa_scores);

        Ok(())
    }

    /// The inner function that maps our song list to Skill Attack's song list and
    /// actually updates the `scores` stored in player. Pulled out here so it can
    /// be reused in both skill attack update methods
    fn update_skill_attack_inner(
        &mut self,
        song_list: Arc<Vec<DDRSong>>,
        sa_scores: HashMap<u16, Scores>,
    ) {
        // create a mapping of skill attack indices to our song index type
        let song_list_index: HashMap<SkillAttackIndex, _> = song_list
            .iter()
            .filter_map(|s| match s.skill_attack_index {
                Some(sa_index) => Some((sa_index, &s.song_id)),
                None => None,
            })
            .collect();
        // get all
        for (song_id, sa_score) in sa_scores.into_iter().filter_map(|(sa_idx, score)| {
            match song_list_index.get(&sa_idx) {
                Some(&song_id) => Some((song_id.to_owned(), score)),
                None => {
                    // old long removed song or new song not in db yet
                    // either way ignore
                    None
                }
            }
        }) {
            self.scores
                .entry(song_id)
                .and_modify(|inner_score| inner_score.update(&sa_score))
                .or_insert(sa_score);
        }
    }

    /// Downloads and updates scores from Sanbai
    ///
    /// Currently returns an error if `Player` does not have a sanbai username
    /// associated with themselves.
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
        let mut score_entry = sanbai_scores_iter.next().unwrap();
        let mut last_song_id = score_entry.song_id.clone();
        let mut combined_score = Scores::default();

        // a do-while loop
        loop {
            if score_entry.song_id == last_song_id {
                combined_score.update_from_sanbai_score_entry(&score_entry);
            } else {
                // push current combined_score to self.scores hashmap
                self.scores
                    .entry(last_song_id)
                    .and_modify(|inner_score| inner_score.update(&combined_score))
                    .or_insert(combined_score);
                // make new combined_score initialized to song_entry
                combined_score = Scores::default();
                combined_score.update_from_sanbai_score_entry(&score_entry);
                // update last_song_id
                last_song_id = score_entry.song_id;
            }
            score_entry = match sanbai_scores_iter.next() {
                Some(s) => s,
                None => {
                    // push current combined_score to self.scores hashmap
                    self.scores
                        .entry(last_song_id)
                        .and_modify(|inner_score| inner_score.update(&combined_score))
                        .or_insert(combined_score);
                    break;
                }
            };
        }

        Ok(())
    }
}
