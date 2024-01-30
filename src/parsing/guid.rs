use serde::{Deserialize, Deserializer};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GUID {
    inner: [u8; 16],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GUIDParsingError {
    #[error("String provided is too short")]
    TooShort,

    #[error("String provided is too long")]
    TooLong,

    #[error("String contains invalid characters")]
    InvalidCharacter,
}

impl TryFrom<&str> for GUID {
    type Error = GUIDParsingError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if s.len() < 32 {
            return Err(GUIDParsingError::TooShort);
        }

        // The additional 4 chars is to account for the possible '-' characters
        if s.len() > 36 {
            return Err(GUIDParsingError::TooLong);
        }

        let mut chars = s.chars();

        let mut inner = [0u8; 16];

        for i in 0..16 {
            let mut byte = 0u8;
            let mut byte_index = 0;
            while byte_index < 2 {
                if let Some(c) = chars.next() {
                    match c {
                        '-' => continue,
                        _ => {
                            if let Some(n) = hex_to_num(c) {
                                // Result of `byte_index ^ 1` is either 1 or 0 and determines
                                // whether the current `n` gets shifted 4 bits to the left.
                                // `byte_index` should be 1 when `byte_index` == 0 and 0 when
                                // `byte_index` == 1
                                byte |= n << 4 * (byte_index ^ 1);
                                byte_index += 1;
                            } else {
                                return Err(GUIDParsingError::InvalidCharacter);
                            }
                        }
                    }
                } else {
                    return Err(GUIDParsingError::TooShort);
                }
            }

            inner[i] = byte;
        }

        Ok(Self { inner })
    }
}

const ASCII_NUMS_START: u32 = 48;
const ASCII_UPPER_ALPHA_START: u32 = 65;
const ASCII_LOWER_ALPHA_START: u32 = 97;

fn hex_to_num(c: char) -> Option<u8> {
    if c as u32 > 127 {
        return None;
    }

    let n = match c {
        '0'..='9' => c as u32 - ASCII_NUMS_START,
        'a'..='f' => c as u32 - ASCII_LOWER_ALPHA_START + 10,
        'A'..='F' => c as u32 - ASCII_UPPER_ALPHA_START + 10,
        _ => return None,
    };

    Some(n as u8)
}

pub(crate) fn deserialize_guid_with_curly_braces<'de, D>(deserializer: D) -> Result<GUID, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s: &str = Deserialize::deserialize(deserializer)?;

    // Ommit the curly braces in the source when parsing
    s = &s[1..s.len() - 1];

    GUID::try_from(s).map_err(|e| serde::de::Error::custom(e))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hex_to_num_ascii_nums() {
        assert_eq!(hex_to_num('0'), Some(0));
        assert_eq!(hex_to_num('5'), Some(5));
        assert_eq!(hex_to_num('9'), Some(9));
    }

    #[test]
    fn hex_to_num_ascii_lower() {
        assert_eq!(hex_to_num('a'), Some(10));
        assert_eq!(hex_to_num('d'), Some(13));
        assert_eq!(hex_to_num('f'), Some(15));
    }

    #[test]
    fn hex_to_num_ascii_upper() {
        assert_eq!(hex_to_num('A'), Some(10));
        assert_eq!(hex_to_num('D'), Some(13));
        assert_eq!(hex_to_num('F'), Some(15));
    }

    #[test]
    fn hex_to_num_invalid_chars() {
        // All ascii chars that are printable
        // NOTE: This is by no means a comprehensive test. This is only used to show that the
        // function `hex_to_num` rejects invalid `char`s
        let invalid_char_iter = ('!'..='/')
            .chain(':'..='@')
            .chain('['..='`')
            .chain('{'..='~');

        invalid_char_iter.for_each(|c| assert_eq!(hex_to_num(c), None));
    }

    #[test]
    fn parse_guid_from_str_with_hyphens() {
        let s = "C7AD875E-1344-4D9B-A883-32E748890908";
        let guid = GUID::try_from(s).expect("Failed to parse GUID");

        let expected = GUID {
            inner: [
                0xC7, 0xAD, 0x87, 0x5E, 0x13, 0x44, 0x4D, 0x9B, 0xA8, 0x83, 0x32, 0xE7, 0x48, 0x89,
                0x09, 0x08,
            ],
        };

        assert_eq!(guid, expected);
    }

    #[test]
    fn parse_guid_from_str_without_hyphens() {
        let s = "C7AD875E13444D9BA88332E748890908";
        let guid = GUID::try_from(s).expect("Failed to parse GUID");

        let expected = GUID {
            inner: [
                0xC7, 0xAD, 0x87, 0x5E, 0x13, 0x44, 0x4D, 0x9B, 0xA8, 0x83, 0x32, 0xE7, 0x48, 0x89,
                0x09, 0x08,
            ],
        };

        assert_eq!(guid, expected);
    }

    #[test]
    fn error_when_parse_guid_from_str_when_too_long() {
        let s = "C7AD875E-1344-4D9B-A883-32E748890908-123321123";

        assert_eq!(GUID::try_from(s), Err(GUIDParsingError::TooLong));
    }

    #[test]
    fn error_when_parse_guid_from_str_when_too_short() {
        let s = "C7AD875E-1344-4D9B-A883";

        assert_eq!(GUID::try_from(s), Err(GUIDParsingError::TooShort));
    }

    #[test]
    fn error_when_parse_guid_from_str_with_invalid_char() {
        let s = "+7AD875E-1344-4D9B-A883-32E748890908";

        assert_eq!(GUID::try_from(s), Err(GUIDParsingError::InvalidCharacter));
    }
}
