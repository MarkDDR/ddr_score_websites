use std::cmp::Reverse;

use anyhow::Result;
use num_format::{Locale, ToFormattedString};
use score_websites::ddr_song::Chart;
use score_websites::search::{parse_search_query, search_by_title, SearchInfo};
// use score_websites::score_websites::sanbai::{get_sanbai_scores, get_sanbai_song_data};
// use score_websites::score_websites::skill_attack;
use score_websites::scores::Player;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    setup();
    let http = score_websites::HttpClient::new();

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

    let db = score_websites::Database::new(http.clone(), users).await?;

    let mut input = String::new();
    loop {
        input.clear();
        println!("\nInput search");
        std::io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");
        let search = input.trim();

        let (search_info, filter) = match parse_search_query(db.song_list(), search) {
            Some(x) => x,
            _ => {
                println!("Please input difficulty");
                continue;
            }
        };

        let search_result = search_by_title(filter, search_info.search_title());
        // let search_result = search_by_title(search_filter, &search);

        let diff_index;
        if let Some(result) = search_result {
            match search_info {
                SearchInfo {
                    chart: Some(c),
                    level: Some(l),
                    ..
                } => {
                    diff_index = c as usize;
                    println!("{} {:?} {}", result.song_name, c, l);
                }
                SearchInfo {
                    chart: Some(c),
                    level: None,
                    ..
                } => {
                    // select the corresponding chart
                    diff_index = c as usize;
                    let chart_index = c as usize;
                    let level = result.ratings.0[chart_index];
                    println!("{} {:?} {}", result.song_name, c, level);
                }
                SearchInfo {
                    chart: None,
                    level: Some(l),
                    ..
                } => {
                    // select corresponding rating
                    let mut chart_index = None;
                    for (index, &rating) in result.ratings.0.iter().enumerate() {
                        if rating == l {
                            chart_index = Some(index);
                            break;
                        }
                    }
                    let chart_index = chart_index.expect("This should be impossible");
                    diff_index = chart_index;
                    let chart = Chart::from_index(chart_index)
                        .expect("This should be even more impossible");
                    println!("{} {:?} {}", result.song_name, chart, l);
                }
                _ => unreachable!("This shouldn't happen"),
            }
            println!("{:#?}", result.search_names);

            let mut user_song_scores = db
                .players()
                .iter()
                .map(|p| {
                    (
                        p.ddr_code,
                        &p.name,
                        p.scores
                            .get(&result.song_id)
                            .and_then(|score| score[diff_index]),
                    )
                })
                .collect::<Vec<_>>();
            user_song_scores.sort_by_key(|(_, _, score_row)| Reverse(score_row.map(|s| s.score)));
            for (code, name, score) in user_song_scores {
                let score_str = match score {
                    None => "-".to_string(),
                    Some(s) => s.score.to_formatted_string(&Locale::en),
                };
                println!("{} | {:8} | {:>9}", code, name, score_str);
            }
            // println!("{:#?}", user_song_scores);
        } else {
            println!("None");
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
