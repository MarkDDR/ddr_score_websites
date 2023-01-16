use std::cmp::Reverse;
use std::time::Duration;

use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use score_websites::scores::{LampType, Player};
use score_websites::search::SearchQuery;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    setup();
    let http = score_websites::HttpClient::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()?;

    let users = [
        (51527130, "MARK", "werecat"),
        (51546306, "TSWIFT", "tSwift"),
        (61578951, "YOSHI", "YOSHI"),
        (61573431, "UNKNOWN", "Melody"),
        (51527333, "KDUBS", "hot_dawg"),
        (51545388, "JAY", "HAPPY HOUR"),
    ]
    .map(|(ddr_code, display_name, sanbai_username)| {
        Player::new(display_name, ddr_code, Some(sanbai_username))
    });

    let db = score_websites::DDRDatabase::new(http.clone(), users).await?;

    let mut input = String::new();
    loop {
        input.clear();
        println!("\nInput search");
        std::io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");
        let search = input.trim();

        let query = match SearchQuery::parse_query(&search, false) {
            Ok(query) => query,
            Err(_) => {
                println!("Error: Not enough arguments");
                println!("USAGE: [song name] [difficulty]");
                continue;
            }
        };

        match query.search(db.song_list()) {
            Some(result) => {
                let mut user_song_scores = db
                    .players()
                    .iter()
                    .map(|p| {
                        (
                            p.ddr_code,
                            &p.name,
                            p.scores
                                .get(&result.song.song_id)
                                .and_then(|score| score[result.chart as usize]),
                        )
                    })
                    .collect::<Vec<_>>();
                user_song_scores
                    .sort_by_key(|(_, _, score_row)| Reverse(score_row.map(|s| s.score)));
                println!(
                    "{} {:?} ({})",
                    &result.song.song_name, result.chart, result.level
                );
                for (code, name, score) in user_song_scores {
                    let (score_str, lamp) = match score {
                        None => ("-".to_string(), ""),
                        Some(s) => (
                            s.score.to_formatted_string(&Locale::en),
                            match s.lamp {
                                LampType::Unknown => "",
                                LampType::Fail => "🇫",
                                LampType::NoCombo => "",
                                LampType::Life4Combo => "🔴",
                                LampType::GoodGreatCombo => "🔵",
                                LampType::GoodCombo => "🔵",
                                LampType::GreatCombo => "🟢",
                                LampType::PerfectCombo => "🟡",
                                LampType::MarvelousCombo => "⚪",
                            },
                        ),
                    };
                    println!("{} | {:8} | {:>9} {}", code, name, score_str, lamp);
                }
            }
            None => println!("Couldn't find that song"),
        }
    }

    // Ok(())
}

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
