use score_websites::website_backends::skill_attack;
use tracing_subscriber::EnvFilter;

fn setup() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup();
    let http = score_websites::HttpClient::new();
    let (sa_scores, poo) = skill_attack::get_scores_and_song_data(http, 51546306).await?;

    let mut input = String::new();
    loop {
        input.clear();
        println!("\nInput search");
        std::io::stdin()
            .read_line(&mut input)
            .expect("Couldn't read line");
        let input = input.trim();
        let num = match input.parse::<u16>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        println!("{:#?}", sa_scores.get(&num));
    }

    // Ok(())
}
