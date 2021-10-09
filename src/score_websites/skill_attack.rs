use std::{borrow::Cow, collections::HashMap};

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use tracing::info;

use crate::scores::{LampType, ScoreRow, Scores};

pub type SkillAttackIndex = u16;

#[derive(Debug, Clone)]
pub struct SkillAttackSong {
    pub skill_attack_index: SkillAttackIndex,
    pub song_name: String,
}

// #[derive(Debug, Clone)]
// pub struct SkillAttackScores {
//     pub ddr_code: u32,
//     pub username: String,
//     pub song_score: HashMap<SkillAttackIndex, Scores>,
// }

pub type SkillAttackScores = HashMap<SkillAttackIndex, Scores>;

pub async fn get_scores(http: Client, ddr_code: u32) -> Result<SkillAttackScores> {
    info!("Sent SA web request");
    let webpage = get_skill_attack_webpage(http, ddr_code).await?;
    let webpage = cut_webpage(&webpage)?;
    info!("got SA webpage");

    let (user_scores, _) = get_scores_and_song_inner(webpage, false)?;

    Ok(user_scores)
}

fn cut_webpage(webpage: &str) -> Result<&str> {
    let s_name_index = webpage
        .find("sName")
        .ok_or(anyhow::anyhow!("couldn't find thing in html"))?;
    let webpage = &webpage[s_name_index..];
    Ok(webpage)
}

pub async fn get_scores_and_song_data(
    http: Client,
    ddr_code: u32,
) -> Result<(SkillAttackScores, Vec<SkillAttackSong>)> {
    info!("Sent SA web request");
    let webpage = get_skill_attack_webpage(http, ddr_code).await?;
    let webpage = cut_webpage(&webpage)?;
    info!("got SA webpage");

    let (user_scores, songs) = get_scores_and_song_inner(webpage, true)?;

    Ok((user_scores, songs))
}

async fn get_skill_attack_webpage(http: Client, ddr_code: u32) -> Result<String> {
    let base = "http://skillattack.com/sa4/dancer_score.php?_=matrix&ddrcode=";
    let url = format!("{}{}", base, ddr_code);

    let webpage = http
        .get(&url)
        .send()
        .await?
        .text_with_charset("Shift_JIS")
        .await?;
    Ok(webpage)
}

fn get_scores_and_song_inner(
    webpage: &str,
    get_songs: bool,
) -> Result<(SkillAttackScores, Vec<SkillAttackSong>)> {
    lazy_static! {
        // A regex that extracts the inside of an Array
        // e.g. "blah blah = new Array(inside part);" will give "inside part"
        static ref INSIDE_ARRAY: Regex = Regex::new(r"Array\((.+)\);$").unwrap();
        // A regex that captures each item that is in single quotes, accounting for escaped single quotes
        // e.g. "'abcd', 'ef\'gh'" will give captures of "abcd" and "ef\'gh"
        static ref QUOTED_TEXT: Regex = Regex::new(r"'(?P<text>(?:[^'\\]|\\.)*)'").unwrap();
    }

    let array_contents = [
        "ddIndex",
        "dsMusic",
        "dsScoreGsp",
        "dsScoreBsp",
        "dsScoreDsp",
        "dsScoreEsp",
        "dsScoreCsp",
        "ddFcGsp",
        "ddFcBsp",
        "ddFcDsp",
        "ddFcEsp",
        "ddFcCsp",
    ]
    .iter()
    // .inspect(|name| println!("{}", name))
    .map(|name| webpage.find(name).unwrap())
    .map(|index| (&webpage[index..]).lines().next().unwrap())
    .map(|line| {
        INSIDE_ARRAY
            .captures(line)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str()
    })
    .collect::<Vec<_>>();

    let song_indices_iter = array_contents[0]
        .split(',')
        .map(|s| s.parse::<SkillAttackIndex>().unwrap());
    let mut song_names_iter = QUOTED_TEXT
        .captures_iter(array_contents[1])
        .map(|cap| cap.name("text").unwrap().as_str())
        .map(|s| decode_html_escapes(s).into_owned());
    let mut scores: Vec<_> = (&array_contents[2..7])
        .iter()
        .map(|s| {
            QUOTED_TEXT
                .captures_iter(s)
                .map(|cap| cap.name("text").unwrap().as_str())
                .map(|s| parse_number_with_commas(s))
        })
        .collect();
    let mut combo_types: Vec<_> = (&array_contents[7..])
        .iter()
        .map(|s| {
            s.split(',').map(|num_str| {
                let combo_index = num_str.parse::<u8>().expect("non number in combo text");
                LampType::from_skill_attack_index(combo_index).unwrap()
            })
        })
        .collect();

    // let username = {
    //     let username_index = webpage.find("sName").unwrap();
    //     let username_line = (&webpage[username_index..]).lines().next().unwrap();
    //     QUOTED_TEXT
    //         .captures(username_line)
    //         .unwrap()
    //         .name("text")
    //         .unwrap()
    //         .as_str()
    //         .to_string()
    // };

    let mut user_scores = HashMap::new();

    let mut skill_attack_songs = vec![];

    info!("Started parsing SA songs");
    for song_index in song_indices_iter {
        // TODO make this cleaner
        let scores = Scores {
            beg_score: scores[0].next().unwrap().map(|s| ScoreRow {
                score: s,
                lamp: combo_types[0].next().unwrap(),
            }),
            basic_score: scores[1].next().unwrap().map(|s| ScoreRow {
                score: s,
                lamp: combo_types[1].next().unwrap(),
            }),
            diff_score: scores[2].next().unwrap().map(|s| ScoreRow {
                score: s,
                lamp: combo_types[2].next().unwrap(),
            }),
            expert_score: scores[3].next().unwrap().map(|s| ScoreRow {
                score: s,
                lamp: combo_types[3].next().unwrap(),
            }),
            chal_score: scores[4].next().unwrap().map(|s| ScoreRow {
                score: s,
                lamp: combo_types[4].next().unwrap(),
            }),
        };
        user_scores.insert(song_index, scores);

        if get_songs {
            let song_name = song_names_iter.next().expect("Song names ended early");
            skill_attack_songs.push(SkillAttackSong {
                skill_attack_index: song_index,
                song_name,
            });
        }
    }
    info!("Finished parsing SA songs");

    Ok((user_scores, skill_attack_songs))
}

fn decode_html_escapes(input: &str) -> Cow<'_, str> {
    match html_escape::decode_html_entities(input) {
        Cow::Borrowed(s) => html_escape::decode_script_single_quoted_text(s),
        Cow::Owned(s) => html_escape::decode_script_single_quoted_text(&s)
            .into_owned()
            .into(),
    }
}

fn parse_number_with_commas(input: &str) -> Option<u32> {
    match input {
        "" | "-" => None,
        _ => Some(
            input
                .bytes()
                .filter(u8::is_ascii_digit)
                .map(|b| b - b'0')
                .fold(0_u32, |n, b| (n * 10) + (b as u32)),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_html_escapes, parse_number_with_commas};

    #[test]
    fn unescape_quote() {
        let text = r"L\'amour et la libert&eacute;(Darwin &amp; DJ Silver remix)";
        let output = decode_html_escapes(text);
        assert_eq!("L'amour et la liberté(Darwin & DJ Silver remix)", output);
    }

    #[test]
    fn parse_numbers() {
        let numbers = ["994,480", "82,010", "1,000,000", "-", "", "3,400", "0"];
        let expected = [
            Some(994_480),
            Some(82_010),
            Some(1_000_000),
            None,
            None,
            Some(3_400),
            Some(0),
        ];
        let output = numbers
            .iter()
            .map(|n| parse_number_with_commas(n))
            .collect::<Vec<_>>();
        assert_eq!(&expected[..], &output[..]);
    }
}
