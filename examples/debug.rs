use score_websites::{
    courses::{Course, CourseSerializeInfo},
    ddr_song::Bpm,
    scores::Player,
    DDRDatabase,
};
use tracing::warn;
use tracing_subscriber::EnvFilter;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

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
    let users = [(51527130, "MARK", "werecat")].map(|(ddr_code, display_name, sanbai_username)| {
        Player::new(display_name, ddr_code, Some(sanbai_username))
    });

    let courses_json = tokio::fs::read_to_string("courses.json").await?;
    let courses_info: Vec<CourseSerializeInfo> = serde_json::from_str(&courses_json)?;

    let http = score_websites::HttpClient::new();
    let db = DDRDatabase::new(http.clone(), users).await?;

    let mut courses = Vec::new();
    for course_info in courses_info {
        let course = Course::new(http.clone(), course_info, db.song_list()).await?;
        courses.push(course);
    }

    let desired_bpm = 500;
    for course in &courses {
        let bpm_width = 7;
        let speed_mod_width = 5;

        println!("{}    (BPM Target: {})\n", course.name, desired_bpm);
        for song in course.songs.iter() {
            match song {
                Some((song, Some(bpm))) => {
                    let speed_mod = speed_mod_calculator(desired_bpm, bpm.get_main_bpm());
                    // let maybe_romanized_name =
                    //     if let Some(romanized_name) = song.romanized_name.as_deref() {
                    //         romanized_name
                    //     } else {
                    //         ""
                    //     };
                    // print!("{} ", pad_discord_string(&song.song_name, name_width));
                    match bpm {
                        Bpm::Constant(bpm) => print!("{:^bpm_width$}", bpm),
                        Bpm::Range { lower, upper, .. } => {
                            print!("{:>bpm_width$}", format!("{}-{}", lower, upper))
                        }
                    }
                    print!(" | {:<speed_mod_width$} | ", format!("{}x", speed_mod));
                    println!("{}", song.song_name);
                }
                Some((song, None)) => {
                    // let maybe_romanized_name =
                    //     if let Some(romanized_name) = song.romanized_name.as_deref() {
                    //         romanized_name
                    //     } else {
                    //         ""
                    //     };
                    println!(
                        "{:^bpm_width$} | {:^speed_mod_width$} | {}",
                        "???", "???", song.song_name
                    );
                }
                None => println!(
                    "{:^bpm_width$} | {:^speed_mod_width$} | Unknown song",
                    "???", "???"
                ),
            }
        }
        println!();
        println!();
    }

    Ok(())
}

// fn main() {
//     let song_name = "サナ・モレッテ・ネ・エンテ";
//     let width = 27;
//     let out = pad_discord_string(song_name, width);
//     println!("{}|hello", out);
//     println!("{}|hello", " ".repeat(width));
// }

// In discord code blocks, full width characters are rendered as 5/3 width
// fn pad_discord_string(input: &str, desired_width: usize) -> String {
//     let mut num_half_width = 0;
//     let mut num_full_width = 0;
//     for c in input.chars() {
//         match c.width() {
//             Some(1) => num_half_width += 1,
//             Some(2) => num_full_width += 1,
//             _ => {}
//         }
//     }

//     let desired_width = desired_width * 3;
//     let mut current_width = num_half_width * 3 + num_full_width * 5;
//     let mut padding = String::new();
//     let full_width_space = '\u{3000}';
//     let half_width_space = " ";
//     while current_width % 3 != 0 {
//         if current_width + 5 > desired_width {
//             warn!("Uh Oh! Couldn't fit into width, giving up");
//             break;
//         }
//         padding.push(full_width_space);
//         current_width += 5;
//     }
//     padding.push_str(&half_width_space.repeat((desired_width - current_width) / 3));

//     format!("{}{}", input, padding)
// }

fn speed_mod_calculator(desired_bpm: u16, song_bpm: u16) -> f64 {
    let ddr_speed_mods = [
        0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0, 3.25, 3.5, 3.75, 4.0,
        4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0,
    ];

    let song_bpm = song_bpm as f64;
    let desired_bpm = desired_bpm as f64;
    let (_, closest_speed_mod) = ddr_speed_mods
        .into_iter()
        .map(|mult| ((song_bpm * mult - desired_bpm).abs(), mult))
        .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap())
        .unwrap();
    closest_speed_mod
}
