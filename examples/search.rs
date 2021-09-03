use std::iter::FromIterator;
use std::str::FromStr;

use anyhow::Result;
use score_websites::ddr_song::{
    filter_by_has_challenge, filter_by_has_non_challenge, filter_by_single_difficulty,
    search_by_title, DDRSong,
};
use score_websites::sanbai;
use score_websites::skill_attack;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    setup();
    let http = reqwest::Client::new();
    let ddr_code = 51527130;
    let ((_user, sa_songs), sanbai_songs) = tokio::try_join!(
        skill_attack::get_scores_and_song(http.clone(), ddr_code),
        sanbai::get_sanbai_song_data(http.clone())
    )?;

    let ddr_songs = DDRSong::from_combining_song_lists(&sanbai_songs, &sa_songs);

    let mut input = String::new();
    loop {
        input.clear();
        println!("\nInput search");
        std::io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");
        let search = input.trim();

        // let potential_params: LastTwo<&str> = search.split_whitespace().skip(1).collect();

        // let mut level_filter;
        // let mut challenge_filter;
        // let mut non_challenge_filter;
        // let search_filter: &mut dyn Iterator<Item = _> = match potential_params {
        //     LastTwo::None => {
        //         println!("Please provide difficulty");
        //         continue;
        //     }
        //     LastTwo::One(a) => match a.parse::<DifficultyOrLevel>() {
        //         Ok(DifficultyOrLevel::Level(x)) => {
        //             level_filter = filter_by_single_difficulty(&ddr_songs, x);
        //             &mut level_filter
        //         }
        //         Ok(DifficultyOrLevel::CSP) => {
        //             challenge_filter = filter_by_has_challenge(&ddr_songs);
        //             &mut challenge_filter
        //         }
        //         Ok(_) => {
        //             non_challenge_filter = filter_by_has_non_challenge(&ddr_songs);
        //             &mut non_challenge_filter
        //         }
        //         Err(_) => {
        //             println!("Please provide difficulty");
        //             continue;
        //         }
        //     },
        //     LastTwo::Two(_, _) => todo!(),
        // };

        let search_result = search_by_title(&ddr_songs, &search);
        // let search_result = search_by_title(search_filter, &search);

        if let Some(result) = search_result {
            println!("{:#?}", result.search_names);
        } else {
            println!("None");
        }
    }

    // Ok(())
}

enum DifficultyOrLevel {
    GSP,
    BSP,
    DSP,
    ESP,
    CSP,
    Level(u8),
}

impl FromStr for DifficultyOrLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DifficultyOrLevel::*;
        match s {
            "gsp" => Ok(GSP),
            "bsp" => Ok(BSP),
            "dsp" => Ok(DSP),
            "esp" => Ok(ESP),
            "csp" => Ok(CSP),
            _ => {
                if let Ok(level) = s.parse::<u8>() {
                    if level < 20 {
                        return Ok(Level(level));
                    }
                }
                Err(())
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LastTwo<T> {
    None,
    One(T),
    Two(T, T),
}

impl<T> FromIterator<T> for LastTwo<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut iter = iter.into_iter();
        let (mut a, mut b) = (None, None);
        while let Some(x) = iter.next() {
            a = b;
            b = Some(x);
        }
        match (a, b) {
            (None, None) => Self::None,
            (None, Some(one)) => Self::One(one),
            (Some(one), Some(two)) => Self::Two(one, two),
            (Some(_), None) => unreachable!(),
        }
    }
}

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
