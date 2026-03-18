use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Keyset pagination cursor — encodes (slot, pos) as "slot:pos".
/// Used in all history endpoints; replaces OFFSET pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SlotPosCursor {
    pub slot: u64,
    pub pos:  u32,
}

impl SlotPosCursor {
    pub fn new(slot: u64, pos: u32) -> Self {
        Self { slot, pos }
    }
}

impl fmt::Display for SlotPosCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.slot, self.pos)
    }
}

#[derive(Debug, Error)]
#[error("invalid cursor format; expected 'slot:pos'")]
pub struct CursorParseError;

impl FromStr for SlotPosCursor {
    type Err = CursorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (slot_str, pos_str) = s.split_once(':').ok_or(CursorParseError)?;
        let slot = slot_str.parse::<u64>().map_err(|_| CursorParseError)?;
        let pos  = pos_str.parse::<u32>().map_err(|_| CursorParseError)?;
        Ok(Self { slot, pos })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_display_parse() {
        let c = SlotPosCursor::new(123_456_789, 42);
        let s = c.to_string();
        assert_eq!(s, "123456789:42");
        let parsed: SlotPosCursor = s.parse().expect("should parse");
        assert_eq!(parsed, c);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!("no-colon".parse::<SlotPosCursor>().is_err());
        assert!("abc:42".parse::<SlotPosCursor>().is_err());
        assert!("123:xyz".parse::<SlotPosCursor>().is_err());
        assert!("".parse::<SlotPosCursor>().is_err());
    }

    #[test]
    fn ordering() {
        let a = SlotPosCursor::new(100, 5);
        let b = SlotPosCursor::new(100, 6);
        let c = SlotPosCursor::new(101, 0);
        assert!(a < b);
        assert!(b < c);
    }
}
