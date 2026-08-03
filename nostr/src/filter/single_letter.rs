use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{Error, ErrorKind};

#[inline]
const fn invalid_len() -> Error {
    Error::with_static_message(
        ErrorKind::Invalid,
        "expected exactly one ASCII alphabetic character",
    )
}

#[inline]
const fn non_ascii_alphabetic_char() -> Error {
    Error::with_static_message(
        ErrorKind::Invalid,
        "expected an ASCII alphabetic character (`a-z` or `A-Z`)",
    )
}

/// A validated single-letter ASCII tag.
///
/// A `SingleLetterTag` can contain exactly one of the following bytes:
///
/// - `b'a'..=b'z'`
/// - `b'A'..=b'Z'`
///
/// The type stores the original character case.
///
/// # Representation
///
/// The value is stored internally as a single [`u8`]. Because the type is
/// `#[repr(transparent)]`, it has the same layout and alignment as `u8`.
///
/// # Invariants
///
/// The inner byte is always an ASCII alphabetic character. This invariant is
/// maintained by keeping the field private and validating every public
/// constructor.
///
/// # Ordering
///
/// ```text
/// a < A < b < B < ... < z < Z
/// ```
///
/// In other words, values are ordered alphabetically without regard to case,
/// with the lowercase variant before the uppercase variant of the same letter.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SingleLetterTag(u8);

impl SingleLetterTag {
    /// The lowercase letter `a`.
    pub const LOWERCASE_A: Self = Self(b'a');

    /// The lowercase letter `b`.
    pub const LOWERCASE_B: Self = Self(b'b');

    /// The lowercase letter `c`.
    pub const LOWERCASE_C: Self = Self(b'c');

    /// The lowercase letter `d`.
    pub const LOWERCASE_D: Self = Self(b'd');

    /// The lowercase letter `e`.
    pub const LOWERCASE_E: Self = Self(b'e');

    /// The lowercase letter `f`.
    pub const LOWERCASE_F: Self = Self(b'f');

    /// The lowercase letter `g`.
    pub const LOWERCASE_G: Self = Self(b'g');

    /// The lowercase letter `h`.
    pub const LOWERCASE_H: Self = Self(b'h');

    /// The lowercase letter `i`.
    pub const LOWERCASE_I: Self = Self(b'i');

    /// The lowercase letter `j`.
    pub const LOWERCASE_J: Self = Self(b'j');

    /// The lowercase letter `k`.
    pub const LOWERCASE_K: Self = Self(b'k');

    /// The lowercase letter `l`.
    pub const LOWERCASE_L: Self = Self(b'l');

    /// The lowercase letter `m`.
    pub const LOWERCASE_M: Self = Self(b'm');

    /// The lowercase letter `n`.
    pub const LOWERCASE_N: Self = Self(b'n');

    /// The lowercase letter `o`.
    pub const LOWERCASE_O: Self = Self(b'o');

    /// The lowercase letter `p`.
    pub const LOWERCASE_P: Self = Self(b'p');

    /// The lowercase letter `q`.
    pub const LOWERCASE_Q: Self = Self(b'q');

    /// The lowercase letter `r`.
    pub const LOWERCASE_R: Self = Self(b'r');

    /// The lowercase letter `s`.
    pub const LOWERCASE_S: Self = Self(b's');

    /// The lowercase letter `t`.
    pub const LOWERCASE_T: Self = Self(b't');

    /// The lowercase letter `u`.
    pub const LOWERCASE_U: Self = Self(b'u');

    /// The lowercase letter `v`.
    pub const LOWERCASE_V: Self = Self(b'v');

    /// The lowercase letter `w`.
    pub const LOWERCASE_W: Self = Self(b'w');

    /// The lowercase letter `x`.
    pub const LOWERCASE_X: Self = Self(b'x');

    /// The lowercase letter `y`.
    pub const LOWERCASE_Y: Self = Self(b'y');

    /// The lowercase letter `z`.
    pub const LOWERCASE_Z: Self = Self(b'z');

    /// The uppercase letter `A`.
    pub const UPPERCASE_A: Self = Self(b'A');

    /// The uppercase letter `B`.
    pub const UPPERCASE_B: Self = Self(b'B');

    /// The uppercase letter `C`.
    pub const UPPERCASE_C: Self = Self(b'C');

    /// The uppercase letter `D`.
    pub const UPPERCASE_D: Self = Self(b'D');

    /// The uppercase letter `E`.
    pub const UPPERCASE_E: Self = Self(b'E');

    /// The uppercase letter `F`.
    pub const UPPERCASE_F: Self = Self(b'F');

    /// The uppercase letter `G`.
    pub const UPPERCASE_G: Self = Self(b'G');

    /// The uppercase letter `H`.
    pub const UPPERCASE_H: Self = Self(b'H');

    /// The uppercase letter `I`.
    pub const UPPERCASE_I: Self = Self(b'I');

    /// The uppercase letter `J`.
    pub const UPPERCASE_J: Self = Self(b'J');

    /// The uppercase letter `K`.
    pub const UPPERCASE_K: Self = Self(b'K');

    /// The uppercase letter `L`.
    pub const UPPERCASE_L: Self = Self(b'L');

    /// The uppercase letter `M`.
    pub const UPPERCASE_M: Self = Self(b'M');

    /// The uppercase letter `N`.
    pub const UPPERCASE_N: Self = Self(b'N');

    /// The uppercase letter `O`.
    pub const UPPERCASE_O: Self = Self(b'O');

    /// The uppercase letter `P`.
    pub const UPPERCASE_P: Self = Self(b'P');

    /// The uppercase letter `Q`.
    pub const UPPERCASE_Q: Self = Self(b'Q');

    /// The uppercase letter `R`.
    pub const UPPERCASE_R: Self = Self(b'R');

    /// The uppercase letter `S`.
    pub const UPPERCASE_S: Self = Self(b'S');

    /// The uppercase letter `T`.
    pub const UPPERCASE_T: Self = Self(b'T');

    /// The uppercase letter `U`.
    pub const UPPERCASE_U: Self = Self(b'U');

    /// The uppercase letter `V`.
    pub const UPPERCASE_V: Self = Self(b'V');

    /// The uppercase letter `W`.
    pub const UPPERCASE_W: Self = Self(b'W');

    /// The uppercase letter `X`.
    pub const UPPERCASE_X: Self = Self(b'X');

    /// The uppercase letter `Y`.
    pub const UPPERCASE_Y: Self = Self(b'Y');

    /// The uppercase letter `Z`.
    pub const UPPERCASE_Z: Self = Self(b'Z');

    /// Creates a tag from an ASCII alphabetic `char`.
    ///
    /// The original character case is preserved.
    ///
    /// # Errors
    ///
    /// Returns an error when `character` is not in `a-zA-Z`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use nostr::filter::SingleLetterTag;
    /// assert_eq!(
    ///     SingleLetterTag::from_char('a'),
    ///     Ok(SingleLetterTag::LOWERCASE_A),
    /// );
    /// assert_eq!(
    ///     SingleLetterTag::from_char('A'),
    ///     Ok(SingleLetterTag::UPPERCASE_A),
    /// );
    /// assert!(SingleLetterTag::from_char('1').is_err());
    /// ```
    #[inline]
    pub const fn from_char(character: char) -> Result<Self, Error> {
        Self::from_byte(character as u8)
    }

    /// Creates a tag from an ASCII alphabetic byte.
    ///
    /// The original character case is preserved.
    ///
    /// # Errors
    ///
    /// Returns an error when `byte`
    /// is not in `b'a'..=b'z'` or `b'A'..=b'Z'`.
    #[inline]
    pub const fn from_byte(byte: u8) -> Result<Self, Error> {
        if byte.is_ascii_alphabetic() {
            Ok(Self(byte))
        } else {
            Err(non_ascii_alphabetic_char())
        }
    }

    /// Returns the underlying ASCII byte.
    #[inline]
    pub const fn as_byte(self) -> u8 {
        self.0
    }

    /// Returns the tag as a `char`.
    #[inline]
    pub const fn as_char(self) -> char {
        self.0 as char
    }

    /// Returns the tag as a one-byte string slice.
    ///
    /// This method does not allocate.
    #[inline]
    pub const fn as_str(&self) -> &str {
        // SAFETY:
        //
        // The private field is initialized only by:
        //
        // - the associated constants, all of which contain ASCII letters;
        // - `from_char`, which validates `is_ascii_alphabetic`;
        // - `from_byte`, which validates `is_ascii_alphabetic`;
        // - the case-normalizing constructors, which validate their input.
        //
        // Every ASCII byte is valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(core::slice::from_ref(&self.0)) }
    }

    /// Returns `true` when the tag contains a lowercase ASCII letter.
    #[inline]
    pub const fn is_lowercase(self) -> bool {
        self.0.is_ascii_lowercase()
    }

    /// Returns `true` when the tag contains an uppercase ASCII letter.
    #[inline]
    pub const fn is_uppercase(self) -> bool {
        self.0.is_ascii_uppercase()
    }

    /// Returns the lowercase form of this tag.
    ///
    /// This method does not modify `self`.
    #[inline]
    #[must_use]
    pub const fn to_lowercase(self) -> Self {
        Self(self.0.to_ascii_lowercase())
    }

    /// Returns the uppercase form of this tag.
    ///
    /// This method does not modify `self`.
    #[inline]
    #[must_use]
    pub const fn to_uppercase(self) -> Self {
        Self(self.0.to_ascii_uppercase())
    }

    /// Compares two tags without considering ASCII letter case.
    #[inline]
    pub const fn eq_ignore_case(self, other: Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }

    /// Produces the key used by the [`Ord`] implementation.
    ///
    /// The first element represents the case-insensitive letter. The second
    /// element is `false` for lowercase and `true` for uppercase.
    #[inline]
    const fn ordering_key(self) -> (u8, bool) {
        (self.0.to_ascii_lowercase(), self.0.is_ascii_uppercase())
    }
}

impl fmt::Debug for SingleLetterTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SingleLetterTag")
            .field(&self.as_char())
            .finish()
    }
}

impl fmt::Display for SingleLetterTag {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl PartialOrd for SingleLetterTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SingleLetterTag {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ordering_key().cmp(&other.ordering_key())
    }
}

impl FromStr for SingleLetterTag {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.as_bytes() {
            &[byte] => Self::from_byte(byte),
            _ => Err(invalid_len()),
        }
    }
}

impl TryFrom<char> for SingleLetterTag {
    type Error = Error;

    #[inline]
    fn try_from(character: char) -> Result<Self, Self::Error> {
        Self::from_char(character)
    }
}

impl TryFrom<u8> for SingleLetterTag {
    type Error = Error;

    #[inline]
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Self::from_byte(byte)
    }
}

impl TryFrom<&str> for SingleLetterTag {
    type Error = Error;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<SingleLetterTag> for char {
    #[inline]
    fn from(tag: SingleLetterTag) -> Self {
        tag.as_char()
    }
}

impl From<SingleLetterTag> for u8 {
    #[inline]
    fn from(tag: SingleLetterTag) -> Self {
        tag.as_byte()
    }
}

impl AsRef<str> for SingleLetterTag {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Serialize for SingleLetterTag {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_char(self.as_char())
    }
}

impl<'de> Deserialize<'de> for SingleLetterTag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let character: char = char::deserialize(deserializer)?;
        Self::from_char(character).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn has_the_same_layout_as_u8() {
        assert_eq!(size_of::<SingleLetterTag>(), size_of::<u8>());
        assert_eq!(align_of::<SingleLetterTag>(), align_of::<u8>());
    }

    #[test]
    fn parses_valid_characters() {
        assert_eq!(
            SingleLetterTag::from_char('a'),
            Ok(SingleLetterTag::LOWERCASE_A),
        );
        assert_eq!(
            SingleLetterTag::from_char('Z'),
            Ok(SingleLetterTag::UPPERCASE_Z),
        );
    }

    #[test]
    fn rejects_invalid_characters() {
        assert_eq!(
            SingleLetterTag::from_char('1'),
            Err(non_ascii_alphabetic_char()),
        );
        assert_eq!(
            SingleLetterTag::from_char('é'),
            Err(non_ascii_alphabetic_char()),
        );
    }

    #[test]
    fn parses_valid_bytes() {
        assert_eq!(
            SingleLetterTag::from_byte(b'p'),
            Ok(SingleLetterTag::LOWERCASE_P),
        );
        assert_eq!(
            SingleLetterTag::from_byte(b'P'),
            Ok(SingleLetterTag::UPPERCASE_P),
        );
    }

    #[test]
    fn parses_valid_strings() {
        assert_eq!(
            "a".parse::<SingleLetterTag>(),
            Ok(SingleLetterTag::LOWERCASE_A),
        );
        assert_eq!(
            "Z".parse::<SingleLetterTag>(),
            Ok(SingleLetterTag::UPPERCASE_Z),
        );
    }

    #[test]
    fn rejects_invalid_strings() {
        assert_eq!("".parse::<SingleLetterTag>(), Err(invalid_len()));
        assert_eq!("ab".parse::<SingleLetterTag>(), Err(invalid_len()));
        assert_eq!("é".parse::<SingleLetterTag>(), Err(invalid_len()));
        assert_eq!(
            "1".parse::<SingleLetterTag>(),
            Err(non_ascii_alphabetic_char()),
        );
    }

    #[test]
    fn exposes_character_representations() {
        let tag = SingleLetterTag::LOWERCASE_P;

        assert_eq!(tag.as_byte(), b'p');
        assert_eq!(tag.as_char(), 'p');
        assert_eq!(tag.as_str(), "p");
        assert_eq!(tag.to_string(), "p");
    }

    #[test]
    fn normalizes_character_case() {
        assert_eq!(
            SingleLetterTag::UPPERCASE_P.to_lowercase(),
            SingleLetterTag::LOWERCASE_P,
        );
        assert_eq!(
            SingleLetterTag::LOWERCASE_P.to_uppercase(),
            SingleLetterTag::UPPERCASE_P,
        );
    }

    #[test]
    fn detects_character_case() {
        assert!(SingleLetterTag::LOWERCASE_A.is_lowercase());
        assert!(!SingleLetterTag::LOWERCASE_A.is_uppercase());

        assert!(SingleLetterTag::UPPERCASE_A.is_uppercase());
        assert!(!SingleLetterTag::UPPERCASE_A.is_lowercase());
    }

    #[test]
    fn compares_case_insensitively() {
        assert!(SingleLetterTag::LOWERCASE_A.eq_ignore_case(SingleLetterTag::UPPERCASE_A));
        assert!(!SingleLetterTag::LOWERCASE_A.eq_ignore_case(SingleLetterTag::UPPERCASE_B));
    }

    #[test]
    fn preserves_the_previous_ordering_semantics() {
        assert!(SingleLetterTag::LOWERCASE_A < SingleLetterTag::UPPERCASE_A);
        assert!(SingleLetterTag::UPPERCASE_A < SingleLetterTag::LOWERCASE_B);
    }

    #[test]
    fn debug_output_contains_the_character() {
        assert_eq!(
            format!("{:?}", SingleLetterTag::LOWERCASE_P),
            "SingleLetterTag('p')",
        );
    }
}
