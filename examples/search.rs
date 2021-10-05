use std::cmp::Reverse;

use anyhow::Result;
use futures::stream::FuturesUnordered;
use num_format::{Locale, ToFormattedString};
use score_websites::ddr_song::{parse_search_query, search_by_title, Chart, DDRSong, SearchInfo};
use score_websites::sanbai::get_sanbai_song_data;
use score_websites::scores::Player;
use score_websites::skill_attack;
use tokio_stream::StreamExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    setup();
    let http = score_websites::Client::new();

    // TODO grab the scores and present them in the search results
    let users = [
        (51527130, "MARK"),
        (51546306, "TSWIFT"),
        (61578951, "YOSHI"),
        (61573431, "CERULEAN"),
        (51527333, "KDUBS"),
        (51545388, "HAPPY HR"),
    ];

    // split users into two futures
    //   - one to get scores and songs from first user
    //   - other to get just scores from the rest of the users
    let (first_user, other_users) = users.split_first().unwrap();
    let sa_songs_and_scores =
        skill_attack::get_scores_and_song_data(http.clone(), first_user.1.to_owned(), first_user.0);
    let other_user_scores = async {
        let mut futures_unordered: FuturesUnordered<_> = other_users
            .iter()
            .copied()
            .map(|(ddr_code, username)| {
                skill_attack::get_scores(http.clone(), username.to_owned(), ddr_code)
            })
            .collect();
        let mut scores = vec![];
        while let Some(score) = futures_unordered.try_next().await? {
            scores.push(score);
        }

        Result::<_, anyhow::Error>::Ok(scores)
    };

    let (sanbai_songs, (first_user_scores, sa_songs), other_user_scores) = tokio::try_join!(
        get_sanbai_song_data(http.clone()),
        sa_songs_and_scores,
        other_user_scores,
    )?;

    let ddr_songs = DDRSong::from_combining_song_lists(&sanbai_songs, &sa_songs);

    let user_scores: Vec<_> = std::iter::once(first_user_scores)
        .chain(other_user_scores.into_iter())
        .map(|sa_score| Player::from_sa_scores(&sa_score, &ddr_songs))
        .collect();

    // TODO Display user scores with results

    let mut input = String::new();
    loop {
        input.clear();
        println!("\nInput search");
        std::io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");
        let search = input.trim();

        let (search_info, filter) = match parse_search_query(&ddr_songs, search) {
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

            let mut user_song_scores = user_scores
                .iter()
                .map(|p| {
                    (
                        p.ddr_code,
                        &p.name,
                        p.scores
                            .get(&result.song_id)
                            .and_then(|score| score.get_by_index(diff_index)),
                    )
                })
                .collect::<Vec<_>>();
            user_song_scores.sort_by_key(|(_, _, score)| Reverse(*score));
            for (code, name, score) in user_song_scores {
                let score_str = match score {
                    None => "-".to_string(),
                    Some(s) => s.to_formatted_string(&Locale::en),
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
