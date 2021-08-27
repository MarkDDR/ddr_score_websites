use tracing::info;

use crate::sanbai::{DDRVersion, Difficulties, LockTypes, SanbaiSong};
use crate::skill_attack::SkillAttackSong;

pub struct DDRSong {
    pub song_id: String,
    pub skill_attack_index: Option<u16>,
    pub song_name: String,
    pub alternative_names: Vec<String>,
    pub version_num: DDRVersion,
    pub deleted: bool,
    pub ratings: Difficulties,
    // Lock condition, i.e. Extra Savior, Golden League, Unlock Event, etc.
    pub lock_types: Option<LockTypes>,
}

impl DDRSong {
    pub fn new_from_sanbai_and_skillattack(
        sanbai: &SanbaiSong,
        skill_attack: Option<&SkillAttackSong>,
    ) -> Self {
        // TODO split the names by '/'
        let alternative_names: Vec<String> = sanbai
            .alternate_name
            .iter()
            .chain(sanbai.romanized_name.iter())
            .chain(sanbai.searchable_name.iter())
            .cloned()
            .collect();
        Self {
            song_id: sanbai.song_id.clone(),
            skill_attack_index: skill_attack.map(|s| s.skill_attack_index),
            song_name: sanbai.song_name.clone(),
            alternative_names,
            version_num: sanbai.version_num,
            deleted: sanbai.deleted,
            ratings: sanbai.ratings,
            lock_types: sanbai.lock_types,
        }
    }

    pub fn from_combining_song_lists(
        sanbai_songs: &[SanbaiSong],
        skill_attack_songs: &[SkillAttackSong],
    ) -> Vec<Self> {
        let sa_normalized = {
            let mut sa_normalized = skill_attack_songs
                .iter()
                .map(|s| (normalize_name(&s.song_name), s))
                .collect::<Vec<_>>();
            sa_normalized.sort_by(|(a, _), (b, _)| a.cmp(b));
            sa_normalized
        };
        let sanbai_normalized = {
            let mut sanbai_normalized = sanbai_songs
                .iter()
                .map(|s| (normalize_name(&s.song_name), s))
                .collect::<Vec<_>>();
            sanbai_normalized.sort_by(|(a, _), (b, _)| a.cmp(b));
            sanbai_normalized
        };

        let mut ddr_songs = vec![];
        let mut sa_index = 0;
        let mut sanbai_index = 0;
        // let mut drop_count = 0;

        loop {
            let (sa_candidate, sanbai_candidate) = match (
                sa_normalized.get(sa_index),
                sanbai_normalized.get(sanbai_index),
            ) {
                (Some(a), Some(b)) => (a, b),
                (None, Some((_, sanbai_song))) => {
                    info!("Leftover song in sanbai: {}", sanbai_song.song_name);
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(sanbai_song, None));
                    sanbai_index += 1;
                    continue;
                }
                _ => break,
            };

            match sa_candidate.0.cmp(&sanbai_candidate.0) {
                std::cmp::Ordering::Equal => {
                    // match, add to vec
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(
                        sanbai_candidate.1,
                        Some(sa_candidate.1),
                    ));
                    sa_index += 1;
                    sanbai_index += 1;
                }
                std::cmp::Ordering::Greater => {
                    // only in sanbai, add to vec
                    info!("Only in Sanbai, adding: {}", sanbai_candidate.1.song_name);
                    ddr_songs.push(Self::new_from_sanbai_and_skillattack(
                        sanbai_candidate.1,
                        None,
                    ));
                    sanbai_index += 1;
                }
                std::cmp::Ordering::Less => {
                    // only in SA, ignore
                    // songs that are only in skill attack are old songs that have been
                    // gone for a long time
                    info!(
                        "Only in Skill Attack, ignoring: {}",
                        sa_candidate.1.song_name
                    );
                    sa_index += 1;
                }
            }
        }

        ddr_songs
    }
}

// Differences between Sanbai and Skill Attack
// - Space between song name and parenteticals `Possession(EDP Mix)`
// - sometimes SA has full width parenthesis, `!`, `+`
// - a couple of smart quotes (over the "period", dreamin')
// - Qipchāq and Qipchãq
// - … and ...
fn normalize_name(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '！' => '!',
            '（' => '(',
            '）' => ')',
            '“' | '”' => '"',
            'ã' | 'ā' => 'a',
            '＋' => '+',
            '’' => '\'',
            _ => c,
        })
        .flat_map(|c| {
            if c == '…' {
                std::iter::repeat('.').take(3)
            } else {
                std::iter::repeat(c).take(1)
            }
        })
        .collect()
}
