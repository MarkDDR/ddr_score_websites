use anyhow::Result;
use score_websites::ddr_song::{parse_search_query, search_by_title, Chart, DDRSong, SearchInfo};
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

        let (search_info, filter) = match parse_search_query(&ddr_songs, search) {
            Some(x) => x,
            _ => {
                println!("Please input difficulty");
                continue;
            }
        };

        let search_result = search_by_title(filter, search_info.search_title());
        // let search_result = search_by_title(search_filter, &search);

        if let Some(result) = search_result {
            match search_info {
                SearchInfo {
                    chart: Some(c),
                    level: Some(l),
                    ..
                } => {
                    println!("{} {:?} {}", result.song_name, c, l);
                }
                SearchInfo {
                    chart: Some(c),
                    level: None,
                    ..
                } => {
                    // select the corresponding chart
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
                    let mut diff_index = None;
                    for (index, &rating) in result.ratings.0.iter().enumerate() {
                        if rating == l {
                            diff_index = Some(index);
                            break;
                        }
                    }
                    let diff_index = diff_index.expect("This should be impossible");
                    let chart =
                        Chart::from_index(diff_index).expect("This should be even more impossible");
                    println!("{} {:?} {}", result.song_name, chart, l);
                }
                _ => unreachable!("This shouldn't happen"),
            }
            // println!("{} {} {}", result.song_name, search_info.)
            println!("{:#?}", result.search_names);
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
