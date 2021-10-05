use std::collections::HashMap;

use crate::{
    ddr_song::{DDRSong, SongId},
    skill_attack::{SkillAttackIndex, SkillAttackScores},
};

#[derive(Debug, Clone, Copy)]
pub struct Scores {
    pub beg_score: Option<u32>,
    pub basic_score: Option<u32>,
    pub diff_score: Option<u32>,
    pub expert_score: Option<u32>,
    pub chal_score: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub name: String,
    pub ddr_code: u32,
    pub scores: HashMap<SongId, Scores>,
}

impl Player {
    pub fn from_sa_scores(sa_scores: &SkillAttackScores, ddr_songs: &[DDRSong]) -> Self {
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
        }
    }
}
