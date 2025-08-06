//! [SemVer Spec](https://semver.org/spec/v2.0.0.html)

pub mod identifiers;
pub mod pre_release;
mod version_extentions;
pub use version_extentions::{Bumpable, Incrementable, Setable};
