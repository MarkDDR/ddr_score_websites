/// DDR song representation and searching
pub mod ddr_song;
/// The backend logic for querying and parsing of DDR score websites
pub mod score_websites;
/// Structures and methods related to storing the scores of players
pub mod scores;

use std::collections::HashMap;

use futures::stream::FuturesUnordered;
/// `reqwest`'s async http client re-exported
pub use reqwest::Client;
use tokio_stream::StreamExt;

use anyhow::Result;
use ddr_song::DDRSong;
use score_websites::sanbai::{get_sanbai_scores, get_sanbai_song_data};
use score_websites::skill_attack;
use scores::Player;

#[derive(Clone, Debug)]
pub struct Database {
    songs: Vec<DDRSong>,
    players: Vec<Player>,
}

impl Database {
    pub async fn new(http: Client, players: impl Into<Vec<Player>>) -> Result<Self> {
        // create tasks for
        //  - sanbai song list,
        //  - sanbai user scores,
        //  - 1 sa song list and user score,
        //  - rest of sa user scores
        // have all those tasks return a enum so I can just match on the variant returned in
        // a FuturesUnordered
        // I don't care about the other sa user scores until the first sa song list comes in
        let mut players: Vec<Player> = players.into();
        let sanbai_song_list = tokio::spawn(get_sanbai_song_data(http.clone()));
        let mut sanbai_user_scores: FuturesUnordered<_> = players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.sanbai_username.clone().map(|name| (i, name)))
            .map(|(i, name)| {
                let http = http.clone();
                tokio::spawn(async move {
                    let scores = get_sanbai_scores(http, &name).await?;
                    Result::<_, anyhow::Error>::Ok((i, scores))
                })
            })
            .collect();
        let sa_song_list = tokio::spawn(skill_attack::get_scores_and_song_data(
            http.clone(),
            players[0].ddr_code,
        ));
        let mut sa_user_scores: FuturesUnordered<_> = players
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, p)| {
                let http = http.clone();
                let ddr_code = p.ddr_code;
                tokio::spawn(async move {
                    let scores = skill_attack::get_scores(http, ddr_code).await?;
                    Result::<_, anyhow::Error>::Ok((i, scores))
                })
            })
            .collect();

        let sanbai_songs = sanbai_song_list.await.expect("task panicked")?;
        // Refactor needed: Player in scores shouldn't do any internet requests,
        // it should just make changes with raw info we provide from our requests

        tokio::pin!(sa_song_list);
        let mut songs = None;
        // await on all the futures and handle each as they finish
        loop {
            tokio::select! {
                skill_attack_songs = &mut sa_song_list, if songs.is_none() => {
                    let (first_player_scores, skill_attack_songs) = skill_attack_songs??;
                    let tmp_songs = DDRSong::from_combining_song_lists(&sanbai_songs, &skill_attack_songs);
                    let first_player = &mut players[0];
                    process_skill_attack_score(first_player, first_player_scores, &tmp_songs);
                    songs = Some(tmp_songs);

                },
                Some(res) = sanbai_user_scores.next() => {
                    let (player_index, sanbai_scores) = res??;
                    let player = &mut players[player_index];
                    // each "score" here actually is just a single "row" of a score,
                    // aka the just the ESP score, or just the BDP score, and in this
                    // vec adjacent difficulty scores are usually next to each other,
                    // so we try to take advantage of that here
                    let mut current_score_entry: Option<(&str, &mut scores::Scores)> = None;
                    for score in &sanbai_scores {
                        match current_score_entry {
                            Some((name, ref mut entry)) if name == score.song_id => {
                                entry.update_from_sanbai_score_entry(score)
                            }
                            _ => {
                                let entry = player.scores.entry(score.song_id.clone()).or_default();
                                entry.update_from_sanbai_score_entry(score);
                                current_score_entry = Some((score.song_id.as_str(), entry));
                            }
                        }
                    }
                },
                Some(res) = sa_user_scores.next(), if songs.is_some() => {
                    let (player_index, sa_scores) = res??;
                    let player = &mut players[player_index];
                    let songs = songs.as_deref().unwrap();
                    process_skill_attack_score(player, sa_scores, songs);
                }
                else => break,
            }
        }
        let songs = songs.unwrap();
        Ok(Self { songs, players })
    }
}

fn process_skill_attack_score(
    player: &mut Player,
    sa_scores: HashMap<u16, scores::Scores>,
    songs: &[DDRSong],
) {
    for (song_id, new_score) in songs
        .iter()
        .filter_map(|s| Some((s.song_id.as_str(), sa_scores.get(&s.skill_attack_index?)?)))
    {
        player
            .scores
            .entry(song_id.to_owned())
            .or_default()
            .update(new_score);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
