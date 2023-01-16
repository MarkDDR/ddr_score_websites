/// Things related to a course in DDR
pub mod courses;
/// DDR song representation and searching
pub mod ddr_song;
/// Error enum
pub mod error;
/// Structures and methods related to storing the scores of players
pub mod scores;
/// Utilities to search the song list for a specific song
pub mod search;
/// The backend logic for querying and parsing of DDR score websites
pub mod website_backends;

use std::collections::HashMap;

use futures::stream::FuturesUnordered;
/// `reqwest`'s async http client re-exported.
pub use reqwest::Client as HttpClient;
use tokio_stream::StreamExt;
use tracing::warn;

use crate::ddr_song::SongId;
use crate::website_backends::sanbai::{get_sanbai_scores, get_sanbai_song_data};
use crate::website_backends::skill_attack;
use ddr_song::DDRSong;
use scores::Player;

pub use error::Result;

/// The main struct of this crate. Handles fetching songs and scores from
/// the different backends and combining them into a single unified format
#[derive(Clone, Debug)]
pub struct DDRDatabase {
    songs: Vec<DDRSong>,
    players: Vec<Player>,
}

impl DDRDatabase {
    /// Creates a new `DDRDatabase` by fetching song lists and scores for the users
    pub async fn new(http: HttpClient, players: impl Into<Vec<Player>>) -> Result<Self> {
        let mut db = Self {
            songs: vec![],
            players: players.into(),
        };
        db.update_scores(http).await?;
        Ok(db)
    }

    /// Updates song list and user scores by fetching them again and updating in place
    /// Returns number new songs and number of new scores
    pub async fn update_scores(&mut self, http: HttpClient) -> Result<(usize, usize)> {
        // create tasks for
        //  - sanbai song list,
        //  - sanbai user scores,
        //  - 1 sa song list and user score,
        //  - rest of sa user scores
        // I don't care about the other sa user scores until the first sa song list comes in
        let sanbai_song_list = tokio::spawn(get_sanbai_song_data(http.clone()));
        let mut sanbai_user_scores: FuturesUnordered<_> = self
            .players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.sanbai_username.clone().map(|name| (i, name)))
            .map(|(i, name)| {
                let http = http.clone();
                tokio::spawn(async move {
                    let scores = get_sanbai_scores(http, &name).await?;
                    Result::Ok((i, scores))
                })
            })
            .collect();
        let sa_song_list = tokio::spawn(skill_attack::get_skill_attack_songs(http.clone()));
        let mut sa_user_scores: FuturesUnordered<_> = self
            .players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let http = http.clone();
                let ddr_code = p.ddr_code;
                tokio::spawn(async move {
                    let scores = skill_attack::get_scores(http, ddr_code).await?;
                    Result::Ok((i, scores))
                })
            })
            .collect();

        let sanbai_songs = sanbai_song_list.await.expect("sanbai song task panicked")?;

        tokio::pin!(sa_song_list);
        let mut sa_songs_updated = false;
        let mut skip_skill_attack = false;
        let mut num_new_songs = 0;
        // FIXME double counting if skill attack score updates first and
        // then sanbai score and sanbai score had more better lamp accuracy
        let mut num_new_scores = 0;
        // await on all the futures and handle each as they finish
        loop {
            tokio::select! {
                // update self.songs with combined skill attack/sanbai list.
                // If skill attack is down, then just update with sanbai list
                skill_attack_songs = &mut sa_song_list, if !sa_songs_updated && !skip_skill_attack => {
                    // TODO handle skill attack being down and skip/update just sanbai songs
                    // TODO Keep old song list in mind and just update entries
                    let skill_attack_songs = match skill_attack_songs.expect("sa song task panicked") {
                        Ok(x) => x,
                        Err(e) => {
                            let song_list_without_skill_attack: Vec<_> = sanbai_songs
                                .iter()
                                .map(|song| DDRSong::new_from_sanbai_and_skillattack(song, None))
                                .collect();
                            self.songs = song_list_without_skill_attack;
                            warn!("ERROR: {:?}", e);
                            warn!("ERROR: Skill attack seems to be down, or has changed its format");
                            skip_skill_attack = true;
                            continue;
                        }
                    };
                    // let skill_attack_songs = skill_attack_songs.expect("sa song task panicked")?;
                    let new_song_list = DDRSong::from_combining_song_lists(&sanbai_songs, &skill_attack_songs);
                    num_new_songs = match new_song_list.len().checked_sub(self.songs.len()) {
                        Some(n) => n,
                        None => {
                            warn!("New song list has fewer songs than old song list!");
                            0
                        }
                    };
                    self.songs = new_song_list;
                    sa_songs_updated = true;

                },
                // add sanbai user score
                Some(res) = sanbai_user_scores.next() => {
                    let (player_index, sanbai_scores) = res.expect("sanbai user score task panicked")?;
                    let player = &mut self.players[player_index];
                    // each "score" here actually is just a single "row" of a score,
                    // aka the just the ESP score, or just the BDP score, and in this
                    // vec adjacent difficulty scores are usually next to each other,
                    // so we try to take advantage of that here
                    let mut current_score_entry: Option<(&SongId, &mut scores::Scores)> = None;
                    for score in &sanbai_scores {
                        match current_score_entry {
                            Some((id, ref mut entry)) if id == &score.song_id => {
                                if entry.update_from_sanbai_score_entry(score) {
                                    num_new_scores += 1;
                                }
                            }
                            _ => {
                                let entry = player.scores.entry(score.song_id.clone()).or_default();
                                if entry.update_from_sanbai_score_entry(score) {
                                    num_new_scores += 1;
                                }
                                current_score_entry = Some((&score.song_id, entry));
                            }
                        }
                    }
                },
                // add skill attack user score
                Some(res) = sa_user_scores.next(), if sa_songs_updated && !skip_skill_attack => {
                    let (player_index, sa_scores) = res.expect("sa user score task panicked")?;
                    let player = &mut self.players[player_index];
                    num_new_scores += process_skill_attack_score(player, sa_scores, &self.songs);
                }
                else => break,
            }
        }
        Ok((num_new_songs, num_new_scores))
    }

    /// A list of all the songs
    pub fn song_list(&self) -> &[DDRSong] {
        &self.songs
    }

    /// A list of the users
    pub fn players(&self) -> &[Player] {
        &self.players
    }
}

// Helper function to reduce code duplication
// Returns the number of scores updated
fn process_skill_attack_score(
    player: &mut Player,
    sa_scores: HashMap<u16, scores::Scores>,
    songs: &[DDRSong],
) -> usize {
    let mut num_new_scores = 0;
    for (song_id, new_score) in songs
        .iter()
        .filter_map(|s| Some((&s.song_id, sa_scores.get(&s.skill_attack_index?)?)))
    {
        num_new_scores += player
            .scores
            .entry(song_id.clone())
            .or_default()
            .update(new_score);
    }
    num_new_scores
}

// #[derive(Debug, Clone)]
// pub struct UpdateInfo {
//     num_new_songs: usize,
//     new_pbs: Vec<PlayerNewPbs>,
// }

// #[derive(Debug, Clone)]
// pub struct PlayerNewPbs {
//     pub player_name: String,
//     pub total_new_pbs: usize,
//     pub num_new_aaas: usize,
//     pub num_new_pfcs: usize,
//     pub num_new_mfcs: usize,
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
