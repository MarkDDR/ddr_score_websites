use crate::ddr_song::{Bpm, DDRSong, SongId};
use crate::error::Error;
use crate::{HttpClient, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSerializeInfo {
    name: String,
    songs: [Option<SongId>; 4],
}

#[derive(Debug, Clone)]
pub struct Course {
    name: String,
    front_songs: [DDRSong; 3],
    last_song: Option<DDRSong>,
    bpms: [Option<Bpm>; 4],
}

impl Course {
    pub async fn new(
        http: HttpClient,
        info: CourseSerializeInfo,
        ddr_songs: &[DDRSong],
    ) -> Result<Self> {
        if info.songs[0..3].iter().any(Option::is_none) {
            return Err(Error::OtherParseError("First 3 songs can't be `None`"));
        }

        let mut songs = Vec::with_capacity(3);
        let mut bpms = [(); 4];

        todo!()
        // for (index, song_id) in info.songs.into_iter().enumerate() {}
    }
}
