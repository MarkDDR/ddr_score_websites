use std::collections::HashMap;

use crate::ddr_song::SongId;
use crate::error::{Error, Result};
use crate::HttpClient;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::result::Result as StdResult;
use tracing::info;

use crate::scores::{LampType, ScoreRow, Scores};

pub type SkillAttackIndex = u16;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct SkillAttackSong {
    pub skill_attack_index: SkillAttackIndex,
    pub song_id: SongId,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub gsp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub bsp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub dsp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub esp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub csp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub bdp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub ddp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub edp: Option<u8>,
    #[serde(deserialize_with = "nonpositive_to_none")]
    pub cdp: Option<u8>,
    #[serde(deserialize_with = "serde_decode_html_escapes")]
    pub song_name: String,
    #[serde(deserialize_with = "serde_decode_html_escapes")]
    pub artist_name: String,
}

fn serde_decode_html_escapes<'de, D>(deserializer: D) -> StdResult<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <&str>::deserialize(deserializer)?;
    Ok(html_escape::decode_html_entities(s).into_owned())
}

fn nonpositive_to_none<'de, D>(deserializer: D) -> StdResult<Option<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let num = <i8>::deserialize(deserializer)?;
    Ok(if num > 0 { Some(num as u8) } else { None })
}

pub async fn get_skill_attack_songs(http: HttpClient) -> Result<Vec<SkillAttackSong>> {
    info!("Fetching Skill Attack song list");
    let url = "http://skillattack.com/sa4/data/master_music.txt";
    let master_list = http
        .get(url)
        .send()
        .await?
        .text_with_charset("Shift_JIS")
        .await?;
    info!("Skill Attack song list fetched");
    let out = parse_skill_attack_tsv(&master_list);
    info!("Skill Attack song list parsed");
    out
}

fn parse_skill_attack_tsv(input: &str) -> Result<Vec<SkillAttackSong>> {
    let mut tsv_reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .quoting(false)
        .from_reader(input.as_bytes());

    Ok(tsv_reader
        .deserialize::<SkillAttackSong>()
        .collect::<StdResult<Vec<_>, _>>()?)
}

pub type SkillAttackScores = HashMap<SkillAttackIndex, Scores>;

pub async fn get_scores(http: HttpClient, ddr_code: u32) -> Result<SkillAttackScores> {
    info!("Sent SA web request");

    let ddr_code = ddr_code;
    let base = "http://skillattack.com/sa4/dancer_score.php?_=matrix&ddrcode=";
    let url = format!("{}{}", base, ddr_code);

    let webpage = http
        .get(&url)
        .send()
        .await?
        .text_with_charset("Shift_JIS")
        .await?;

    let webpage = cut_webpage(&webpage)?;
    info!("got SA webpage");

    let user_scores = get_scores_inner(webpage)?;

    Ok(user_scores)
}

pub fn cut_webpage(webpage: &str) -> Result<&str> {
    let s_name_index = webpage
        .find("sName")
        .ok_or(Error::OtherParseError("couldn't find \"sName\" in html"))?;
    let webpage = &webpage[s_name_index..];
    Ok(webpage)
}

// This code is so ugly, I'm sorry
// Maybe this can get replaced with a better more robust parser in the future
pub fn get_scores_inner(webpage: &str) -> Result<SkillAttackScores> {
    // A regex that extracts the inside of an Array
    // e.g. "blah blah = new Array(inside part);" will give "inside part"
    static INSIDE_ARRAY: Lazy<Regex> = Lazy::new(|| Regex::new(r"Array\((.+)\);$").unwrap());
    // A regex that captures each item that is in single quotes, accounting for escaped single quotes
    // e.g. "'abcd', 'ef\'gh'" will give captures of "abcd" and "ef\'gh"
    static QUOTED_TEXT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"'(?P<text>(?:[^'\\]|\\.)*)'").unwrap());

    let array_contents = [
        "ddIndex",
        "dsScoreGsp",
        "dsScoreBsp",
        "dsScoreDsp",
        "dsScoreEsp",
        "dsScoreCsp",
        "dsScoreBdp",
        "dsScoreDdp",
        "dsScoreEdp",
        "dsScoreCdp",
        "ddFcGsp",
        "ddFcBsp",
        "ddFcDsp",
        "ddFcEsp",
        "ddFcCsp",
        "ddFcBdp",
        "ddFcDdp",
        "ddFcEdp",
        "ddFcCdp",
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

    let song_indices = array_contents[0]
        .split(',')
        .map(|s| {
            s
                // .trim()
                .parse::<SkillAttackIndex>()
                .map_err(|_| Error::SkillAttackHtmlParseError("index parse"))
        })
        .collect::<Result<Vec<_>>>()?;

    let scores: Vec<Vec<_>> = (&array_contents[1..10])
        .iter()
        .map(|s| {
            QUOTED_TEXT
                .captures_iter(s)
                .map(|cap| {
                    cap.name("text")
                        .map(|s| parse_number_with_commas(s.as_str()))
                        .ok_or(Error::SkillAttackHtmlParseError("score regex match"))
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<Vec<_>>>>()?;
    let combo_types: Vec<Vec<_>> = (&array_contents[10..])
        .iter()
        .map(|s| {
            s.split(',')
                .map(|num_str| {
                    let combo_index = num_str
                        // .trim()
                        .parse::<u8>()
                        .map_err(|_| {
                            Error::SkillAttackHtmlParseError("combo type num wasn't u8")
                        })?;
                    LampType::from_skill_attack_index(combo_index).ok_or(
                        Error::SkillAttackHtmlParseError("Unrecognized skill attack lamp type"),
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<Vec<_>>>>()?;

    let mut user_scores = HashMap::new();

    if !scores
        .iter()
        .map(|v| v.len())
        .chain(combo_types.iter().map(|v| v.len()))
        .all(|l| l == song_indices.len())
    {
        return Err(Error::SkillAttackHtmlParseError(
            "Array lengths didn't match!",
        ));
    }

    info!("Started parsing SA songs");
    for (i, song_index) in song_indices.into_iter().enumerate() {
        let score_rows = [0, 1, 2, 3, 4, 5, 6, 7, 8].map(|diff_index| {
            scores[diff_index][i].map(|s| ScoreRow {
                score: s,
                lamp: combo_types[diff_index][i],
                time_played: None,
            })
        });

        let scores = Scores {
            beg_score: score_rows[0],
            basic_score: score_rows[1],
            diff_score: score_rows[2],
            expert_score: score_rows[3],
            chal_score: score_rows[4],
            doubles_basic_score: score_rows[5],
            doubles_diff_score: score_rows[6],
            doubles_expert_score: score_rows[7],
            doubles_chal_score: score_rows[8],
        };
        user_scores.insert(song_index, scores);
    }
    info!("Finished parsing SA songs");

    Ok(user_scores)
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
    use super::{parse_number_with_commas, parse_skill_attack_tsv, SkillAttackSong};

    #[test]
    fn parse_tsv() {
        let input_tsv = r#"246	ddib8P601q0Oqdb0Pl8oqobq9DD608P1	1	2	3	4	5	6	7	8	9	L'amour et la libert&eacute;(Darwin &amp; DJ Silver remix)	NAOKI in the MERCURE
63	olQQ8QPPqqObDD9ooodOl9i9od8b06I9	5	7	11	13	-1	8	10	14	-1	Healing Vision ～Angelic mix～	2MB
74	1Qqo9bID18OdiI1Qb0lP9DIqIO6ldPOP	4	5	7	10	-1	4	6	10	-1	CANDY&#9825;	小坂りゆ
818	Pq1O0qIiQII9PP1Qi6dbi9Pdo88dO8Dq	4	7	12	16	-1	7	12	16	-1	ANNIVERSARY &there4;∵&there4; &larr;&darr;&uarr;&rarr;	BEMANI Sound Team "U1 overground""#;
        let expected_output = [
            SkillAttackSong {
                skill_attack_index: 246,
                song_id: "ddib8P601q0Oqdb0Pl8oqobq9DD608P1".parse().unwrap(),
                gsp: Some(1),
                bsp: Some(2),
                dsp: Some(3),
                esp: Some(4),
                csp: Some(5),
                bdp: Some(6),
                ddp: Some(7),
                edp: Some(8),
                cdp: Some(9),
                song_name: "L'amour et la liberté(Darwin & DJ Silver remix)".into(),
                artist_name: "NAOKI in the MERCURE".into(),
            },
            SkillAttackSong {
                skill_attack_index: 63,
                song_id: "olQQ8QPPqqObDD9ooodOl9i9od8b06I9".parse().unwrap(),
                gsp: Some(5),
                bsp: Some(7),
                dsp: Some(11),
                esp: Some(13),
                csp: None,
                bdp: Some(8),
                ddp: Some(10),
                edp: Some(14),
                cdp: None,
                song_name: "Healing Vision ～Angelic mix～".into(),
                artist_name: "2MB".into(),
            },
            SkillAttackSong {
                skill_attack_index: 74,
                song_id: "1Qqo9bID18OdiI1Qb0lP9DIqIO6ldPOP".parse().unwrap(),
                gsp: Some(4),
                bsp: Some(5),
                dsp: Some(7),
                esp: Some(10),
                csp: None,
                bdp: Some(4),
                ddp: Some(6),
                edp: Some(10),
                cdp: None,
                song_name: "CANDY♡".into(),
                artist_name: "小坂りゆ".into(),
            },
            SkillAttackSong {
                skill_attack_index: 818,
                song_id: "Pq1O0qIiQII9PP1Qi6dbi9Pdo88dO8Dq".parse().unwrap(),
                gsp: Some(4),
                bsp: Some(7),
                dsp: Some(12),
                esp: Some(16),
                csp: None,
                bdp: Some(7),
                ddp: Some(12),
                edp: Some(16),
                cdp: None,
                song_name: "ANNIVERSARY ∴∵∴ ←↓↑→".into(),
                artist_name: r#"BEMANI Sound Team "U1 overground""#.into(),
            },
        ];
        let output = parse_skill_attack_tsv(input_tsv).unwrap();
        assert_eq!(output.len(), 4);
        for (actual, expected) in output.into_iter().zip(expected_output) {
            assert_eq!(actual, expected);
        }
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
