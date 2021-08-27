use anyhow::Result;
use score_websites::ddr_song::DDRSong;
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

    // let mut input = String::new();
    // loop {
    //     input.clear();
    //     println!("\nInput search");
    //     io::stdin()
    //         .read_line(&mut input)
    //         .expect("Couldn't read line");
    //     let search = input.trim();

    //     let candidates: Vec<_> = songs
    //         .iter()
    //         .map(|s| &s.song_name)
    //         .filter(|s| s.contains(search))
    //         .collect();

    //     println!("{:#?}", candidates);
    // }

    Ok(())
}

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}
