use miette::{IntoDiagnostic, LabeledSpan, bail, ensure, miette};
use semver::{BuildMetadata, Prerelease, Version};
use std::str::FromStr;
use tracing::instrument;

use crate::{Action, Result, current_span, error::VersionError};
pub trait Bumpable {
    /// Used to bump the version then set the [`Prerelease`] and [`BuildMetadata`].
    fn bump(
        &mut self,
        action: Action,
        pre_release: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force_version: bool,
    ) -> Result<Version>;

    fn try_bump_pre(&mut self, force: bool) -> Result<Version>;
    fn try_bump_patch(&mut self) -> Result<Version>;
    fn try_bump_minor(&mut self, force: bool) -> Result<Version>;
    fn try_bump_major(&mut self, force: bool) -> Result<Version>;
}

impl Bumpable for Version {
    #[instrument(fields(from, to), skip(self))]
    fn bump(
        &mut self,
        action: Action,
        pre_release: Option<Prerelease>,
        build: Option<BuildMetadata>,
        force_version: bool,
    ) -> Result<Version> {
        use crate::cli::Action as BumpVer;
        let span = current_span!();
        let old_version = self.clone();
        span.record("from", self.to_string());
        tracing::trace!("Bumping version");
        match action {
            BumpVer::Patch => self.try_bump_patch()?,
            BumpVer::Minor => self.try_bump_minor(force_version)?,
            BumpVer::Major => self.try_bump_major(force_version)?,
            BumpVer::Pre => self.try_bump_pre(force_version)?,
            _ => bail!("Invalid Action: {}", action),
        };
        if action != Action::Pre {
            if let Some(pre) = pre_release {
                self.pre = pre
            }
        };

        if let Some(build) = build {
            self.build = build
        }
        if !force_version {
            ensure!(
                self.clone() > old_version,
                "New version is not large than old version"
            );
        }
        let ver_str = &self.clone().to_string();
        span.record("to", ver_str);
        tracing::debug!("Version bumped to: {}", ver_str);
        Ok(self.clone())
    }

    fn try_bump_patch(&mut self) -> Result<Version> {
        let old_version = self.clone();
        let version = self;
        if version.pre.is_empty() {
            version.patch += 1;
        } else {
            version.pre = semver::Prerelease::EMPTY
        }

        ensure!(
            &old_version < version,
            "Patch bump error: old={}, new={}",
            old_version.to_string(),
            version.clone().to_string()
        );
        Ok(version.clone())
    }

    fn try_bump_minor(&mut self, force: bool) -> Result<Version> {
        let old_version = self.clone();
        let version = self;
        if !version.pre.is_empty() && !force {
            Err(VersionError::prerelease_not_empty(
                &old_version,
                Action::Minor,
            ))?;
        }
        version.pre = Prerelease::EMPTY;
        version.minor += 1;
        version.patch = 0;
        ensure!(&old_version < version, "Failed to bump: Minor");
        Ok(version.clone())
    }

    fn try_bump_major(&mut self, force: bool) -> Result<Version> {
        let old_version = self.clone();
        let version = self;
        if !version.pre.is_empty() && !force {
            Err(VersionError::prerelease_not_empty(
                &old_version,
                Action::Minor,
            ))?;
        }
        version.pre = Prerelease::EMPTY;
        version.major += 1;
        version.minor = 0;
        version.patch = 0;
        ensure!(&old_version < version, "Failed to bump: Major");
        Ok(version.clone())
    }
    ///
    /// Pre-release must be in form `-<ascii>.<number>`
    /// TODO: What to do when not set. Should we match on a -> b -> rc
    #[instrument(skip(self), fields(from, to))]
    fn try_bump_pre(&mut self, force: bool) -> Result<Version> {
        let span = current_span!();
        span.record("from", self.to_string());
        tracing::trace!("Bumping pre");
        let old_version = self.clone();
        if self.pre == Prerelease::EMPTY {
            bail!(
                help = "Set the pre-release either with the pre-release flag `--pre` or using the `set` action.",
                "Prerelease not set."
            );
        }

        if let Some((id, num)) = self.pre.to_string().split_once('.') {
            let num = match u64::from_str(num) {
                Ok(n) => n + 1,
                Err(e) => {
                    let source_code = self.pre.to_string();
                    let split_idx = source_code.find('.').expect("Already checked");
                    let mut span = (
                        split_idx,
                        source_code.chars().count() as isize - 1isize - split_idx as isize,
                    );
                    if span.1 < 0 {
                        span.1 = 0isize
                    }

                    let mut span = (span.0, span.1 as usize);
                    span.0 += self.to_string().find('-').unwrap() + 2 + 11;
                    let labels = vec![LabeledSpan::new(
                        Some("Invalid incrementable segment".into()),
                        span.0,
                        span.1,
                    )];
                    let err = miette!(
                        labels = labels,
                        help = "Must only be a number after the '.' in prerelease segment.",
                        "When converting to int: {e}"
                    )
                    .with_source_code(format!(r#"version = "{}""#, self.to_string()));

                    return Err(err);
                }
            };
            let pre = Prerelease::new(&format!("{id}.{num}")).into_diagnostic()?;
            self.pre = pre
        } else {
            bail!("Prerelease not split by '.': {}", self.pre.to_string());
        };

        ensure!(self.clone() > old_version, "PreRelease bump failed.");
        span.record("to", self.to_string());
        tracing::debug!("Prerelease bumped.");

        Ok(self.clone())
    }
}

pub trait Setable {
    fn set_version(&mut self, new_version: Version) -> Result<Version>;
    fn set_prerelease(&mut self, new_prerelease: Prerelease) -> Result<Version>;
}
impl Setable for Version {
    #[instrument(skip_all, fields(from, to))]
    fn set_version(&mut self, new_version: Version) -> Result<Version> {
        let span = current_span!();
        span.record("from", self.to_string());
        span.record("to", new_version.to_string());
        tracing::debug!("Setting version");

        self.build = new_version.build.clone();
        self.pre = new_version.pre.clone();
        self.patch = new_version.patch;
        self.minor = new_version.minor;
        self.major = new_version.major;

        miette::ensure!(self.clone() == new_version, "Failed to set version");
        Ok(self.clone())
    }

    #[instrument(skip_all, fields(from, to))]
    fn set_prerelease(&mut self, new_prerelease: Prerelease) -> Result<Version> {
        let span = current_span!();
        span.record("from", self.to_string());
        tracing::trace!("set_prerelease");
        self.pre = new_prerelease;
        span.record("to", self.to_string());
        tracing::debug!("Set new prerelease");
        Ok(self.clone())
    }
}

#[allow(dead_code)]
/// TODO: Implement for any incrimentable type within reason.
pub trait Incrimentable {
    fn increment(&mut self);
    fn increment_by(&mut self, n: isize);
}

// #[cfg(test)]
// mod tests {
//     use semver::BuildMetadata;
//     use semver::Prerelease;

//     use super::*;

//     macro_rules! version {
//         ($maj:literal $min:literal $pat:literal) => {
//             Version::new($maj, $min, $pat)
//         };
//         ($maj:literal $min:literal $pat:literal -$pre:ident) => {{
//             let mut v = version!($maj $min $pat);
//             v.pre = semver::Prerelease::new(stringify!($pre)).unwrap_or_default();
//             v
//         }};
//         ($maj:literal $min:literal $pat:literal -$pre:ident +$build:ident) => {{
//             let mut v = version!($maj $min $pat -$pre);
//             v.build = semver::BuildMetadata::new(stringify!($build)).unwrap_or_default();
//             v
//         }};
//         ($ver:literal) => {
//             semver::Version::parse($ver).unwrap()
//         }
//     }
//     #[test]
//     fn test_version_macro() {
//         let basic = Version::new(1, 1, 0);
//         assert_eq!(basic, version!(1 1 0));

//         let mut basic_with_pre = basic.clone();
//         basic_with_pre.pre = semver::Prerelease::new("alpha").unwrap();
//         assert_eq!(basic_with_pre, version!(1 1 0 -alpha));
//         let mut basic_with_pre_and_build = basic_with_pre.clone();
//         basic_with_pre_and_build.build = BuildMetadata::new("test").unwrap();
//         assert_ne!(basic_with_pre, basic_with_pre_and_build);
//         assert_eq!(basic_with_pre_and_build, version!(1 1 0 -alpha +test));
//     }

//     #[test]
//     fn bump_patch_simple() {
//         let mut version = version!(0 1 1);

//         bump_patch(&mut version);
//         assert_eq!(version, version!(0 1 2), "Bump 0.1.1 -> 0.1.2");
//         bump_patch(&mut version);
//         assert_eq!(version, version!(0 1 3), "Bump 0.1.2 -> 0.1.3");
//     }

//     #[test]
//     fn bump_patch_pre() {
//         let mut version = version!("0.1.1-alpha.2");

//         assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
//         bump_patch(&mut version);
//         assert_eq!(version, version!(0 1 1));
//         assert!(version > version!("0.1.1-alpha.2"));
//     }

//     #[test]
//     fn bump_minor_simple() {
//         let mut version = version!(0 1 1);

//         try_bump_minor(&mut version, false).unwrap();
//         assert_eq!(version, version!(0 2 0), "Bump 0.1.1 -> 0.2.0");
//         try_bump_minor(&mut version, false).unwrap();
//         assert_eq!(version, version!(0 3 0), "Bump 0.2.0 -> 0.3.0");
//     }

//     #[test]
//     fn bump_minor_pre_force() {
//         let mut version = version!("0.1.1-alpha.2");

//         assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
//         try_bump_minor(&mut version, true).unwrap();
//         assert_eq!(version, version!(0 2 0), "Bump 0.1.1-alpha.2 -> 0.2.0");
//         assert!(version > version!("0.1.1-alpha.2"));
//     }
//     #[test]
//     #[should_panic]
//     fn bump_minor_pre_no_force() {
//         let mut version = version!("0.1.1-alpha.2");

//         assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
//         try_bump_minor(&mut version, false).unwrap();
//     }

//     #[test]
//     fn bump_major_simple() -> miette::Result<()> {
//         let mut version = version!(0 1 1);

//         try_bump_major(&mut version, false)?;
//         assert_eq!(version, version!(1 0 0), "Bump 0.1.1 -> 1.0.0");
//         try_bump_major(&mut version, false)?;
//         assert_eq!(version, version!(2 0 0), "Bump 1.0.0 -> 2.0.0");
//         Ok(())
//     }

//     #[test]
//     fn bump_major_pre() -> miette::Result<()> {
//         let mut version = version!("0.1.1-alpha.2");

//         assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
//         try_bump_major(&mut version, true)?;
//         assert_eq!(version, version!(1 0 0), "Bump 0.1.1-alpha.2 -> 1.0.0");
//         assert!(version > version!("0.1.1-alpha.2"));
//         Ok(())
//     }

//     #[test]
//     #[should_panic]
//     fn bump_major_pre_no_force() {
//         let mut version = version!("0.1.1-alpha.2");

//         assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
//         try_bump_major(&mut version, false).unwrap();
//     }
// }
