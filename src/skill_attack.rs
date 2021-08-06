use std::{borrow::Cow, collections::HashMap};

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;

type SkillAttackIndex = u16;

#[derive(Debug, Clone)]
pub struct SkillAttackSong {
    pub skill_attack_index: SkillAttackIndex,
    pub song_name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct SkillAttackScore {
    pub beg_score: Option<u32>,
    pub basic_score: Option<u32>,
    pub diff_score: Option<u32>,
    pub expert_score: Option<u32>,
    pub chal_score: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UserScores {
    pub user_id: u32,
    pub user_name: String,
    pub song_score: HashMap<SkillAttackIndex, SkillAttackScore>,
}

pub async fn get_scores(http: Client, ddr_code: u32) -> Result<UserScores> {
    let webpage = get_skill_attack_webpage(http, ddr_code).await?;
    let s_name_index = webpage
        .find("sName")
        .ok_or(anyhow::anyhow!("couldn't find thing in html"))?;
    let webpage = &webpage[s_name_index..];

    let (user_scores, _) = get_scores_and_song_inner(webpage, ddr_code, false)?;

    Ok(user_scores)
}

pub async fn get_scores_and_song(
    http: Client,
    ddr_code: u32,
) -> Result<(UserScores, Vec<SkillAttackSong>)> {
    println!("Sent web request");
    let webpage = get_skill_attack_webpage(http, ddr_code).await?;
    let s_name_index = webpage
        .find("sName")
        .ok_or(anyhow::anyhow!("couldn't find thing in html"))?;
    let webpage = &webpage[s_name_index..];

    println!("got webpage");
    let (user_scores, songs) = get_scores_and_song_inner(webpage, ddr_code, true)?;

    Ok((user_scores, songs.expect("Unexpectedly None")))
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
    ddr_code: u32,
    get_songs: bool,
) -> Result<(UserScores, Option<Vec<SkillAttackSong>>)> {
    lazy_static! {
        static ref INSIDE_ARRAY: Regex = Regex::new(r"Array\((.+)\);$").unwrap();
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
        .captures_iter(&array_contents[1])
        .map(|cap| cap.name("text").unwrap().as_str())
        .map(|s| decode_html_escapes(s).into_owned());
    let scores: Vec<_> = (&array_contents[2..])
        .iter()
        .map(|s| {
            QUOTED_TEXT
                .captures_iter(s)
                .map(|cap| cap.name("text").unwrap().as_str())
                .map(|s| parse_number_with_commas(s))
                .collect::<Vec<_>>()
        })
        .collect();

    let mut user_scores = UserScores {
        user_id: ddr_code,
        user_name: "Bob".to_string(),
        song_score: HashMap::new(),
    };
    let mut song_names = if get_songs { Some(vec![]) } else { None };

    println!("started loop");
    for (score_index, song_index) in song_indices_iter.enumerate() {
        let scores = SkillAttackScore {
            beg_score: scores[0][score_index],
            basic_score: scores[1][score_index],
            diff_score: scores[2][score_index],
            expert_score: scores[3][score_index],
            chal_score: scores[4][score_index],
        };
        user_scores.song_score.insert(song_index, scores);

        if get_songs {
            let song_name = song_names_iter.next().expect("Song names ended early");
            song_names = song_names.map(|mut v| {
                v.push(SkillAttackSong {
                    skill_attack_index: song_index,
                    song_name: song_name,
                });
                v
            });
        }
    }
    println!("ended loop");

    Ok((user_scores, song_names))
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
