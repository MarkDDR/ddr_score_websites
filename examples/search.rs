use anyhow::Result;
use score_websites::skill_attack;

#[tokio::main]
async fn main() -> Result<()> {
    let http = reqwest::Client::new();
    let ddr_code = 51527130;
    let (user, songs) = skill_attack::get_scores_and_song(http, ddr_code).await?;
    // println!("{:#?}", &user);
    // println!("{:#?}", &songs);

    Ok(())
}
