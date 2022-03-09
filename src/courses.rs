use crate::ddr_song::{Bpm, DDRSong, SongId};
use crate::{HttpClient, Result};
use futures::stream::FuturesOrdered;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSerializeInfo {
    pub name: String,
    pub songs: [Option<SongId>; 4],
}

#[derive(Debug, Clone)]
pub struct Course {
    pub name: String,
    pub songs: Vec<Option<(DDRSong, Option<Bpm>)>>,
}

impl Course {
    pub async fn new(
        http: HttpClient,
        info: CourseSerializeInfo,
        ddr_songs: &[DDRSong],
    ) -> Result<Self> {
        let mut songs = Vec::with_capacity(4);

        let mut fut: FuturesOrdered<_> = info
            .songs
            .into_iter()
            .map(|id| {
                let ddr_song = match id {
                    Some(id) => ddr_songs.iter().find(|s| s.song_id == id).cloned(),
                    None => None,
                };
                let http = http.clone();
                async move {
                    match ddr_song {
                        Some(ddr_song) => {
                            let bpm = ddr_song.fetch_bpm(http).await?;
                            Result::<_>::Ok(Some((ddr_song, bpm)))
                        }
                        None => Ok(None),
                    }
                }
            })
            .collect();

        while let Some(res) = fut.try_next().await? {
            songs.push(res);
        }

        Ok(Self {
            name: info.name,
            songs,
        })
    }
}
