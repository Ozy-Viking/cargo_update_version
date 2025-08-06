use std::{
    borrow::Borrow,
    fmt::{Display, Formatter},
    num::ParseIntError,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use crate::Incrementable;

static VALID_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-";
static VALID_DIGITS: &str = "0123456789";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IdentifierKind {
    Alphanumeric,
    Numeric,
}

impl IdentifierKind {
    /// Returns `true` if the identifier kind is [`Alphanumeric`].
    ///
    /// [`Alphanumeric`]: IdentifierKind::Alphanumeric
    #[must_use]
    pub fn is_alphanumeric(&self) -> bool {
        matches!(self, Self::Alphanumeric)
    }

    /// Returns `true` if the identifier kind is [`Numeric`].
    ///
    /// [`Numeric`]: IdentifierKind::Numeric
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    kind: IdentifierKind,
    ident: String,
}

impl Identifier {
    /// Returns `true` if the identifier is [`Alphanumeric`].
    ///
    /// [`Alphanumeric`]: Identifier::Alphanumeric
    #[must_use]
    pub fn is_alphanumeric(&self) -> bool {
        self.kind().is_alphanumeric()
    }

    /// Returns `true` if the identifier is [`Numeric`].
    ///
    /// [`Numeric`]: Identifier::Numeric
    #[must_use]
    pub fn is_numeric(&self) -> bool {
        self.kind().is_numeric()
    }

    pub fn as_alphanumeric(&self) -> Option<&String> {
        if self.kind().is_alphanumeric() {
            Some(&self.ident)
        } else {
            None
        }
    }

    pub fn as_numeric(&self) -> Option<u64> {
        if self.kind().is_numeric() {
            Some(u64::from_str(&self.ident).expect("ensured when set"))
        } else {
            None
        }
    }

    /// Returns the Identifier Kind
    pub fn kind(&self) -> IdentifierKind {
        self.kind
    }
}

impl AsRef<str> for Identifier {
    fn as_ref(&self) -> &str {
        &self.ident
    }
}

impl FromStr for Identifier {
    type Err = IdentifierError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdentifierError::EmptyIdent);
        }

        Identifier::validate_input(s)?;

        if s.chars().all(|c| c.is_ascii_digit()) {
            u64::from_str(s).map_err(IdentifierError::from)?;
            Ok(Self {
                kind: IdentifierKind::Numeric,
                ident: s.to_string(),
            })
        } else {
            Ok(Self {
                kind: IdentifierKind::Alphanumeric,
                ident: s.to_string(),
            })
        }
    }
}

impl Identifier {
    /// Checks whether the input is valid.
    ///
    /// Returns an error [`IdentifierError::InvalidChar`] on the first [`char`]
    pub fn validate_input(input: &str) -> Result<(), IdentifierError> {
        for (idx, c) in input.chars().enumerate() {
            if !(VALID_CHARS.contains(c) | VALID_DIGITS.contains(c)) {
                return Err(IdentifierError::InvalidChar(c, idx));
            }
        }
        Ok(())
    }
}

impl Incrementable for Identifier {
    #[track_caller]
    fn increment(&mut self) {
        self.increment_by(1)
    }

    #[track_caller]
    fn increment_by(&mut self, n: u64) {
        match self.kind() {
            IdentifierKind::Alphanumeric => {
                let new = Alphanumeric::new(&self.ident).expect("already validated");
                self.ident = new.to_string();
            }
            IdentifierKind::Numeric => {
                let new = u64::from_str(&self.ident).expect("Always from u64") + n;
                self.ident = new.to_string();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Numeric(u64);

impl Deref for Numeric {
    type Target = u64;

    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl DerefMut for Numeric {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct Alphanumeric(Vec<AsciiType>);

impl Display for Alphanumeric {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let chars = self
            .as_ref()
            .iter()
            .map(|c| u8::try_from(*c.as_ref()).expect("Valid ascii"))
            .collect::<Vec<_>>();
        write!(f, "{}", String::from_utf8(chars).unwrap())
    }
}

impl AsRef<Vec<AsciiType>> for Alphanumeric {
    fn as_ref(&self) -> &Vec<AsciiType> {
        &self.0
    }
}

impl Alphanumeric {
    fn new(input: &str) -> Result<Alphanumeric, IdentifierError> {
        let mut vec = Vec::new();
        for (idx, c) in input.chars().enumerate() {
            vec.push(AsciiType::from_char(c, idx)?);
        }
        Ok(Alphanumeric(vec))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsciiType {
    UpperAscii(char),
    LowerAscii(char),
    Number(char),
}

impl AsciiType {
    pub fn from_char(c: char, pos: usize) -> Result<AsciiType, IdentifierError> {
        if c.is_ascii_digit() {
            Ok(AsciiType::Number(c))
        } else if c.is_ascii_lowercase() {
            Ok(AsciiType::LowerAscii(c))
        } else if c.is_ascii_uppercase() {
            Ok(AsciiType::UpperAscii(c))
        } else {
            Err(IdentifierError::InvalidChar(c, pos))
        }
    }
}

impl Deref for AsciiType {
    type Target = char;

    fn deref(&self) -> &Self::Target {
        match self {
            AsciiType::UpperAscii(c) | AsciiType::LowerAscii(c) | AsciiType::Number(c) => c,
        }
    }
}

impl AsRef<char> for AsciiType {
    fn as_ref(&self) -> &char {
        match self {
            AsciiType::UpperAscii(c) | AsciiType::LowerAscii(c) | AsciiType::Number(c) => c,
        }
    }
}

impl AsciiType {
    /// Returns `true` if the ascii type is [`UpperAscii`].
    ///
    /// [`UpperAscii`]: AsciiType::UpperAscii
    #[must_use]
    fn is_upper_ascii(&self) -> bool {
        matches!(self, Self::UpperAscii(..))
    }

    fn as_upper_ascii(&self) -> Option<&char> {
        if let Self::UpperAscii(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the ascii type is [`LowerAscii`].
    ///
    /// [`LowerAscii`]: AsciiType::LowerAscii
    #[must_use]
    fn is_lower_ascii(&self) -> bool {
        matches!(self, Self::LowerAscii(..))
    }

    fn as_lower_ascii(&self) -> Option<&char> {
        if let Self::LowerAscii(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the ascii type is [`Number`].
    ///
    /// [`Number`]: AsciiType::Number
    #[must_use]
    fn is_number(&self) -> bool {
        matches!(self, Self::Number(..))
    }

    fn as_number(&self) -> Option<&char> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl Borrow<char> for AsciiType {
    fn borrow(&self) -> &char {
        self.as_ref()
    }
}

impl Borrow<str> for Identifier {
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}
#[derive(Debug, Clone, thiserror::Error, miette::Diagnostic, PartialEq, Eq)]
pub enum IdentifierError {
    #[error("Ident can't be empty.")]
    EmptyIdent,
    /// Invalid character at 0-indexed position
    #[error("Invalid character: {0}")]
    InvalidChar(char, usize),
    #[error("Expected to Numeric")]
    ExpectedNumeric,
    #[error("{0}")]
    ParseIntError(ParseIntError),
}

impl From<ParseIntError> for IdentifierError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseIntError(value)
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Identifier {
    /// 1. Identifiers consisting of only digits are compared numerically.
    ///
    /// 2. Identifiers with letters or hyphens are compared lexically in ASCII sort order.
    ///
    /// 3. Numeric identifiers always have lower precedence than non-numeric identifiers.
    ///
    ///4. A larger set of pre-release fields has a higher precedence than a smaller set, if all of the preceding identifiers are equal.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use IdentifierKind as IdKind;
        use std::cmp::Ordering;
        // if let Identifier::Numeric(self_num) = self {
        //     if let Identifier::Numeric(other_num) = other {
        //         return self_num.cmp(other_num);
        //     } else {
        //         return Ordering::Less;
        //     }
        // } else {
        //     if other.is_numeric() {
        //         return Ordering::Greater;
        //     }
        // }

        match (self.kind(), other.kind()) {
            (IdKind::Alphanumeric, IdKind::Alphanumeric) => {
                self.ident.chars().cmp(other.ident.chars())
            }
            (IdKind::Alphanumeric, IdKind::Numeric) => Ordering::Greater,
            (IdKind::Numeric, IdKind::Alphanumeric) => Ordering::Less,
            (IdKind::Numeric, IdKind::Numeric) => {
                self.as_numeric().unwrap().cmp(&other.as_numeric().unwrap())
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use super::*;
    use Identifier as Ident;
    use IdentifierError as IdentErr;
    use IdentifierKind as Kind;

    fn numeric(pre: impl Display) -> Ident {
        Ident {
            kind: Kind::Numeric,
            ident: pre.to_string(),
        }
    }

    fn alpha(pre: impl Into<String>) -> Ident {
        Ident {
            kind: Kind::Alphanumeric,
            ident: pre.into(),
        }
    }

    #[test]
    pub fn ident_eq() {
        assert!(numeric(1) == numeric(1));
        assert!(numeric(1) != numeric(2));
        assert!(alpha("test") == alpha("test"));
        assert!(alpha("test1") != alpha("test2"));
        assert!(numeric(1) != alpha("test2"));
    }

    #[test]
    pub fn ident_cmp() {
        assert!(numeric(2) > numeric(1));
        assert!(numeric(1) < numeric(2));
        assert!(alpha("alpha") < alpha("beta"));
        assert!(alpha("beta") < alpha("rc"));
        assert!(alpha("RC") < alpha("rc"));
        assert!(alpha("-rc") < alpha("rc"));
        assert!(numeric(2) < alpha("rc"));
        assert!(alpha("rc") > numeric(2));
    }

    #[test]
    pub fn validate_input() {
        assert!(Ident::validate_input("asnjhfksa").is_ok());
        assert!(Ident::validate_input("@shdajkldsha").is_err());
        assert_eq!(
            Ident::validate_input("@!shdajkldsha").unwrap_err(),
            IdentifierError::InvalidChar('@', 0)
        )
    }

    #[test]
    pub fn from_str() {
        assert_eq!(Ident::from_str("1").unwrap(), numeric(1));
        assert_eq!(Ident::from_str("alpha").unwrap(), alpha("alpha"));
        assert_eq!(Ident::from_str("1a").unwrap(), alpha("1a"));
        assert_eq!(
            Ident::from_str("1a@").unwrap_err(),
            IdentErr::InvalidChar('@', 2)
        );
        assert_eq!(Ident::from_str("alpha").unwrap(), alpha("alpha"));
    }
}
