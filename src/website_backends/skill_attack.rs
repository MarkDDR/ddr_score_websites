use std::{borrow::Cow, collections::HashMap};

use crate::error::{Error, Result};
use crate::HttpClient;
use once_cell::sync::Lazy;
use regex::Regex;
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

pub async fn get_scores(http: HttpClient, ddr_code: u32) -> Result<SkillAttackScores> {
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
        .ok_or(Error::OtherParseError("couldn't find \"sName\" in html"))?;
    let webpage = &webpage[s_name_index..];
    Ok(webpage)
}

pub async fn get_scores_and_song_data(
    http: HttpClient,
    ddr_code: u32,
) -> Result<(SkillAttackScores, Vec<SkillAttackSong>)> {
    info!("Sent SA web request");
    let webpage = get_skill_attack_webpage(http, ddr_code).await?;
    let webpage = cut_webpage(&webpage)?;
    info!("got SA webpage");

    let (user_scores, songs) = get_scores_and_song_inner(webpage, true)?;

    Ok((user_scores, songs))
}

async fn get_skill_attack_webpage(http: HttpClient, ddr_code: u32) -> Result<String> {
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

// This code is so ugly, I'm sorry
// Maybe this can get replaced with a better more robust parser in the future
fn get_scores_and_song_inner(
    webpage: &str,
    get_songs: bool,
) -> Result<(SkillAttackScores, Vec<SkillAttackSong>)> {
    // A regex that extracts the inside of an Array
    // e.g. "blah blah = new Array(inside part);" will give "inside part"
    static INSIDE_ARRAY: Lazy<Regex> = Lazy::new(|| Regex::new(r"Array\((.+)\);$").unwrap());
    // A regex that captures each item that is in single quotes, accounting for escaped single quotes
    // e.g. "'abcd', 'ef\'gh'" will give captures of "abcd" and "ef\'gh"
    static QUOTED_TEXT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"'(?P<text>(?:[^'\\]|\\.)*)'").unwrap());

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
    .map(|name| {
        webpage
            .find(name)
            .ok_or(Error::SkillAttackHtmlParseError(name))
    })
    .map(|index| index.map(|index| (&webpage[index..]).lines().next().unwrap()))
    .map(|line| {
        INSIDE_ARRAY
            .captures(line?)
            .ok_or(Error::SkillAttackHtmlParseError("array regex capture"))?
            .get(1)
            .map(|s| s.as_str())
            .ok_or(Error::SkillAttackHtmlParseError("array regex match"))
    })
    .collect::<Result<Vec<_>>>()?;

    let song_indices_iter = array_contents[0].split(',').map(|s| {
        s.parse::<SkillAttackIndex>()
            .map_err(|_| Error::SkillAttackHtmlParseError("index parse"))
    });
    let mut song_names_iter = QUOTED_TEXT.captures_iter(array_contents[1]).map(|cap| {
        cap.name("text")
            .map(|s| decode_html_escapes(s.as_str()).into_owned())
            .ok_or(Error::SkillAttackHtmlParseError("song name regex match"))
    });

    let mut scores: Vec<_> = (&array_contents[2..7])
        .iter()
        .map(|s| {
            QUOTED_TEXT.captures_iter(s).map(|cap| {
                cap.name("text")
                    .map(|s| parse_number_with_commas(s.as_str()))
                    .ok_or(Error::SkillAttackHtmlParseError("score regex match"))
            })
        })
        .collect();
    let mut combo_types: Vec<_> = (&array_contents[7..])
        .iter()
        .map(|s| {
            s.split(',').map(|num_str| {
                let combo_index = num_str
                    .parse::<u8>()
                    .map_err(|_| Error::SkillAttackHtmlParseError("combo type num wasn't u8"))?;
                LampType::from_skill_attack_index(combo_index).ok_or(
                    Error::SkillAttackHtmlParseError("Unrecognized skill attack lamp type"),
                )
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
        let song_index = song_index?;
        // TODO make this cleaner
        let g_score = scores[0].next().ok_or(Error::SkillAttackHtmlParseError(
            "beginner score ended early",
        ))??;
        let g_lamp = combo_types[0]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError(
                "beginner lamps ended early",
            ))??;
        let b_score = scores[1]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError("basic score ended early"))??;
        let b_lamp = combo_types[1]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError("basic lamps ended early"))??;
        let d_score = scores[2].next().ok_or(Error::SkillAttackHtmlParseError(
            "difficult score ended early",
        ))??;
        let d_lamp = combo_types[2]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError(
                "difficult lamps ended early",
            ))??;
        let e_score = scores[3]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError("expert score ended early"))??;
        let e_lamp = combo_types[3]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError("expert lamps ended early"))??;
        let c_score = scores[4].next().ok_or(Error::SkillAttackHtmlParseError(
            "challenge score ended early",
        ))??;
        let c_lamp = combo_types[4]
            .next()
            .ok_or(Error::SkillAttackHtmlParseError(
                "challenge lamps ended early",
            ))??;
        let scores = Scores {
            beg_score: g_score.map(|s| ScoreRow {
                score: s,
                lamp: g_lamp,
            }),
            basic_score: b_score.map(|s| ScoreRow {
                score: s,
                lamp: b_lamp,
            }),
            diff_score: d_score.map(|s| ScoreRow {
                score: s,
                lamp: d_lamp,
            }),
            expert_score: e_score.map(|s| ScoreRow {
                score: s,
                lamp: e_lamp,
            }),
            chal_score: c_score.map(|s| ScoreRow {
                score: s,
                lamp: c_lamp,
            }),
        };
        user_scores.insert(song_index, scores);

        if get_songs {
            let song_name = song_names_iter
                .next()
                .ok_or(Error::SkillAttackHtmlParseError("song names ended early"))??;
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

// TODO error or saturate if we try to parse a number bigger than 2^32
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
