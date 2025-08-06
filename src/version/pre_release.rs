//! ## Semantic Versioning Prerelease
//!
//! - A pre-release version MAY be denoted by appending a hyphen and a series of dot separated identifiers
//!     immediately following the patch version.
//! - Identifiers MUST comprise only ASCII alphanumerics and hyphens [0-9A-Za-z-].
//! - Identifiers MUST NOT be empty.
//! - Numeric identifiers MUST NOT include leading zeroes.
//!
//! Pre-release versions have a lower precedence than the associated normal version. A pre-release
//! version indicates that the version is unstable and might not satisfy the intended compatibility
//! requirements as denoted by its associated normal version.
//!
//! Examples: 1.0.0-alpha, 1.0.0-alpha.1, 1.0.0-0.3.7, 1.0.0-x.7.z.92, 1.0.0-x-y-z.--   
//!
//! Precedence for two pre-release versions with the same major, minor, and patch version MUST be determined by comparing each dot separated identifier from left to right until a difference is found as follows:
//!
//!    1. Identifiers consisting of only digits are compared numerically.
//!
//!    2. Identifiers with letters or hyphens are compared lexically in ASCII sort order.
//!
//!    3. Numeric identifiers always have lower precedence than non-numeric identifiers.
//!
//!    4. A larger set of pre-release fields has a higher precedence than a smaller set,
//!     if all of the preceding identifiers are equal.
//!
//! Example: 1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0
//!
//! ## Backus–Naur Form Grammar for Valid SemVer Versions
//!
//! ```text
//! <pre-release> ::= <dot-separated pre-release identifiers>
//!
//! <dot-separated pre-release identifiers> ::= <pre-release identifier>
//!                                              | <pre-release identifier> "." <dot-separated pre-release identifiers>
//!
//! <pre-release identifier> ::= <alphanumeric identifier>
//!                           | <numeric identifier>
//!
//! <alphanumeric identifier> ::= <non-digit>
//!                             | <non-digit> <identifier characters>
//!                             | <identifier characters> <non-digit>
//!                             | <identifier characters> <non-digit> <identifier characters>
//!
//! <numeric identifier> ::= "0"
//!                        | <positive digit>
//!                        | <positive digit> <digits>
//!
//! <identifier characters> ::= <identifier character>
//!                           | <identifier character> <identifier characters>
//!
//! <identifier character> ::= <digit>
//!                          | <non-digit>
//!
//! <non-digit> ::= <letter>
//!               | "-"
//!
//! <digits> ::= <digit>
//!            | <digit> <digits>
//!
//! <digit> ::= "0"
//!           | <positive digit>
//! ```
//!
//! ## References
//!
//! [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html)
use std::{fmt::Display, marker::PhantomData, ops::Deref, str::FromStr};

use semver::Prerelease;

use crate::{Incrementable, Result, version::identifiers::Identifier};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// Prerelease able to be Bumped.
pub struct PreBumpable;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// Unable to be bumped or incremented.
pub struct PreStatic;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// Enable prerelease bumping
pub struct Pre<PreType> {
    prerelease: Vec<Identifier>,
    _type: PhantomData<PreType>,
}

impl<PreType> Pre<PreType> {
    pub fn new(pre: impl Into<String>) -> Result<Pre<PreType>> {
        let mut prerelease = Vec::new();
        for field in pre.into().split('.') {
            prerelease.push(Identifier::from_str(field)?);
        }
        Ok(Self {
            prerelease,
            _type: PhantomData::<PreType>,
        })
    }
}

impl<PreType> AsRef<Vec<Identifier>> for Pre<PreType> {
    fn as_ref(&self) -> &Vec<Identifier> {
        &self.prerelease
    }
}

impl<PreType> Deref for Pre<PreType> {
    type Target = Vec<Identifier>;

    fn deref(&self) -> &Self::Target {
        &self.prerelease
    }
}

impl<PreType> Display for Pre<PreType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prerelease.join("."))
    }
}

impl Pre<PreStatic> {
    #[must_use]
    pub fn is_bumpable(&self) -> bool {
        false
    }

    #[must_use]
    pub fn is_static(&self) -> bool {
        !self.is_bumpable()
    }
}

impl Pre<PreBumpable> {
    #[must_use]
    pub fn is_bumpable(&self) -> bool {
        true
    }
    #[must_use]
    pub fn is_static(&self) -> bool {
        !self.is_bumpable()
    }
}

impl Incrementable for Pre<PreBumpable> {
    #[track_caller]
    /// Increment the last field by 1.
    ///
    /// Panics on error.
    fn increment(&mut self) {
        let last = self.len() - 1;
        self.increment_field(last)
            .expect("Last field is should always present.");
    }

    #[track_caller]
    /// Increment the last field by m.
    ///
    /// Panics on error.
    fn increment_by(&mut self, m: u64) {
        let last = self.len() - 1;
        self.increment_field_by(last, m)
            .expect("Last field is should always present.");
    }
}

impl Pre<PreBumpable> {
    /// Increment field n by 1 value.
    fn increment_field(&mut self, n: usize) -> Result<(), PreError> {
        self.increment_field_by(n, 1)
    }

    /// Increment field n by m values.
    fn increment_field_by(&mut self, n: usize, m: u64) -> Result<(), PreError> {
        if let Some(field) = self.prerelease.get_mut(n) {
            field.increment_by(m);
            Ok(())
        } else {
            Err(PreError::NoField(n))
        }
    }
}

#[derive(Debug, Clone, thiserror::Error, miette::Diagnostic, PartialEq, Eq)]
pub enum PreError {
    #[error("No field at index: {0}")]
    NoField(usize),
}

impl<PreType> From<Prerelease> for Pre<PreType> {
    fn from(value: Prerelease) -> Self {
        Self::new(value.as_str()).expect("Coming from semver package")
    }
}

impl<PreType> From<Pre<PreType>> for Prerelease {
    fn from(value: Pre<PreType>) -> Self {
        Prerelease::new(&value.to_string()).expect("Moving from 1 validated package to anouther")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_prerelease() {
        let prerelease_str = "alpha.beta.1";
        let prerelease = Pre::<PreStatic>::new(prerelease_str).expect("set for test");
        let prerelease_2 = Pre::<PreStatic>::new("not.equal").expect("set for test");
        assert_eq!(&prerelease.to_string(), prerelease_str);
        assert_ne!(&prerelease_2.to_string(), prerelease_str);
    }

    #[test]
    fn is_bumpable() {
        let bumpable = Pre {
            prerelease: vec![Identifier::from_str("1").unwrap()],
            _type: PhantomData::<PreBumpable>,
        };
        let non_bumpable = Pre {
            prerelease: vec![Identifier::from_str("1").unwrap()],
            _type: PhantomData::<PreStatic>,
        };

        assert!(bumpable.is_bumpable());
        assert!(!non_bumpable.is_bumpable())
    }

    #[test]
    fn new() {
        let bumpable = Pre::<PreBumpable>::new("1").unwrap();
        let non_bumpable = Pre::<PreStatic>::new("1").unwrap();

        assert!(bumpable.is_bumpable());
        assert!(!bumpable.is_static());
        assert!(non_bumpable.is_static());
        assert!(!non_bumpable.is_bumpable());
    }

    #[test]
    fn increment_last_field_by_1() {
        let mut bumpable = Pre::<PreBumpable>::new("1").unwrap();
        let bumped = Pre::<PreBumpable>::new("2").unwrap();

        bumpable.increment();
        assert_eq!(bumpable, bumped)
    }

    #[test]
    fn increment_last_field_by_n() {
        let mut bumpable = Pre::<PreBumpable>::new("1").unwrap();
        let bumped = Pre::<PreBumpable>::new("5").unwrap();

        bumpable.increment_by(4);
        assert_eq!(bumpable, bumped)
    }

    #[test]
    fn increment_n_field_by_m() {
        let mut bumpable = Pre::<PreBumpable>::new("1.1.1.1.1").unwrap();
        let bumped = Pre::<PreBumpable>::new("1.5.1.1.1").unwrap();

        bumpable.increment_field_by(1, 4).unwrap();
        assert_eq!(bumpable, bumped);
        assert_eq!(bumpable.to_string(), bumped.to_string());
    }

    #[test]
    fn increment_n_field_by_1() {
        let mut bumpable = Pre::<PreBumpable>::new("1.1.1.1.1").unwrap();
        let bumped = Pre::<PreBumpable>::new("1.2.1.1.1").unwrap();

        bumpable.increment_field(1).unwrap();
        assert_eq!(bumpable, bumped);
        assert_eq!(bumpable.to_string(), bumped.to_string());
    }

    #[test]
    fn moving_between_prerelease_type() {
        let pre_str = "1.1.1.1.1";
        let pre = Pre::<PreBumpable>::new(pre_str).unwrap();
        let expected: Prerelease = Prerelease::new(pre_str).unwrap();
        let prerelease: Prerelease = pre.clone().into();
        assert_eq!(expected, prerelease);
        assert_eq!(pre, expected.into());
        assert_eq!(prerelease.as_str(), pre_str)
    }
}
