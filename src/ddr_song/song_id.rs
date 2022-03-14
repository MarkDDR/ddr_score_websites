use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

const ALPHABET: &[u8; 16] = b"01689DIOPQbdiloq";

/// A compact opaque representation of a song id, used for comparisons.
/// This takes advantage of the song id string being encoded as hex but in a different alphabet
/// than is normally used to compactly store it as a `u128`
///
/// # Examples
///
/// ```
/// use score_websites::ddr_song::SongId;
///
/// let song_id_str = "bIlqP91O9ld1lqlq6qoq9OiPdqIDPP0l";
/// let other_id_str = "QD9Ib18D9lIO10O19d16PbPb68q1190d";
///
/// let song_id: SongId = song_id_str.parse().unwrap();
/// let song_id_2: SongId = song_id_str.parse().unwrap();
/// let other_id: SongId = other_id_str.parse().unwrap();
///
/// assert_eq!(song_id, song_id_2);
/// assert_ne!(song_id, other_id);
///
/// assert_eq!(song_id.to_string(), song_id_str);
/// assert_eq!(other_id.to_string(), other_id_str);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SongId {
    bytes: u128,
}

impl FromStr for SongId {
    type Err = SongIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            return Err(SongIdParseError::InvalidLength(s.len()));
        }
        let mut bytes = 0_u128;
        for (shift, byte) in s.bytes().enumerate().map(|(i, b)| ((i as u128 * 4), b)) {
            let hex_pos = match ALPHABET.iter().position(|&b| b == byte) {
                Some(pos) => pos,
                None => return Err(SongIdParseError::InvalidChar(byte as char)),
            };
            bytes |= (hex_pos as u128) << shift;
        }

        Ok(Self { bytes })
    }
}

impl Display for SongId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::with_capacity(32);
        let mask = 0xF;
        for alphabet_index in (0..32).map(|x| (self.bytes >> (x * 4)) & mask) {
            let c = ALPHABET[alphabet_index as usize] as char;
            out.push(c);
        }

        write!(f, "{}", out)
    }
}

impl<'de> Deserialize<'de> for SongId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for SongId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // TODO can be more efficient if we make a stack str instead
        let string = self.to_string();
        serializer.serialize_str(&string)
    }
}

#[derive(Error, Debug, Clone)]
pub enum SongIdParseError {
    #[error("The string contained invalid character `{0}`")]
    InvalidChar(char),
    #[error("Expected length 32 string, found length {0} string")]
    InvalidLength(usize),
}

#[cfg(test)]
mod tests {
    use super::SongId;
    #[test]
    fn song_id_serialize_deserialize() {
        let input = [
            "6P18lOliIQqIO6Di0PP8iDlDQ01b0o0q",
            "0bq9qI9PoPIlQl89bDO60o9q8I1iIP66",
            "ld6P1lbb0bPO9doqbbPOoPb8qoDo8id0",
            "qOlDPoiqibIOqod69dPilbiqD6qdO1qQ",
        ];
        for id in input {
            let song_id: SongId = id.parse().unwrap();
            assert_eq!(song_id.to_string(), id);
        }
    }

    #[test]
    fn song_id_comparison() {
        let input = [
            "6P18lOliIQqIO6Di0PP8iDlDQ01b0o0q",
            "0bq9qI9PoPIlQl89bDO60o9q8I1iIP66",
            "ld6P1lbb0bPO9doqbbPOoPb8qoDo8id0",
            "qOlDPoiqibIOqod69dPilbiqD6qdO1qQ",
        ]
        .map(|s| s.parse::<SongId>().unwrap());

        for (x, id) in input.iter().enumerate() {
            for (y, other) in input.iter().enumerate() {
                if x == y {
                    assert_eq!(id, other);
                } else {
                    assert_ne!(id, other);
                }
            }
        }
    }

    #[test]
    fn invalid_ids() {
        let input = [
            // invalid bytes in various positions
            "6P18lOliIQqIO6Di0AP8iDlDQ01b0o0q",
            "6P18lOliIQqIO6Di0PP8iDlDQ01b0o0W",
            "ZP18lOliIQqIO6Di0PP8iDlDQ01b0o0q",
            // invalid lengths
            "6P18lOliIQqIO6Di0PP8iDlDQ01b0o0qq",
            "6P18lOliIQqIO6Di0PP8iDlDQ01b0o0",
            "",
            "6",
        ];

        for id in input {
            id.parse::<SongId>().unwrap_err();
        }
    }
}
