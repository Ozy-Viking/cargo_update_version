pub(crate) mod toml_file;

use miette::Result;
use semver::{Prerelease, Version};
use tracing::{info, instrument, warn};

use crate::{
    Action, Packages, cli, current_span,
    error::{ManifestNotFoundError, VersionError},
};

#[instrument(fields(cargo_file))]
pub fn generate_packages(args: &cli::Cli) -> Result<Packages> {
    let mut cli_path = args.manifest.manifest_path.as_ref();
    let mut command = args.manifest.metadata();
    if let Some(manifest_path) = cli_path.as_mut() {
        let mut manifest_path = manifest_path.clone();
        if manifest_path.is_dir() {
            manifest_path.push("Cargo.toml");
        }
        command.manifest_path(manifest_path);
    };
    let metadata = match command.exec() {
        Ok(m) => m,
        Err(e) => {
            use cargo_metadata::Error as CmErr;
            let mut msg = match e {
                CmErr::CargoMetadata { stderr: s } => s,
                CmErr::Io(e) => e.to_string(),
                CmErr::Utf8(utf8_error) => utf8_error.to_string(),
                CmErr::ErrUtf8(_) => "Cargo.toml is not in valid Utf-8.".into(),
                CmErr::Json(error) => error.to_string(),
                CmErr::NoJson => CmErr::NoJson.to_string(),
            };
            msg.retain(|s| s != '\n');
            let source_code = cli_path.as_ref().map(|&s| {
                s.clone()
                    .canonicalize()
                    .unwrap_or(s.clone())
                    .display()
                    .to_string()
                    .strip_prefix("\\\\?\\") // Bug fix
                    .unwrap_or_default()
                    .to_string()
            });
            let source_len = source_code.clone().unwrap_or_default().len();

            Err(ManifestNotFoundError {
                help: "Ensure you run the command in a rust project or use '--manifest-path' to set a Cargo.toml file".into(),
                source_code,
                label: Some((0, source_len)),
                label_msg: "Check path ends with Cargo.toml or one exists in the given folder.",
                msg,
            })?;
            unreachable!()
        }
    };
    let mut packages = Packages::from(&metadata);
    let cargo_file = metadata.workspace_root.clone().join("Cargo.toml");
    tracing::Span::current().record("cargo_file", cargo_file.clone().to_string());
    if cargo_file.exists() {
        info!("Found")
    } else {
        warn!("Not Found");
        // TODO: Validate this error.
        Err(ManifestNotFoundError {
            help: "Cargo could not find the Cargo.toml file, try specifying it with '-P'.".into(),
            source_code: None,
            label: None,
            label_msg: "",
            msg: "Cargo.toml file was not found by Cargo".into(),
        })?;
    }

    packages.set_cargo_file_path(cargo_file.into());

    Ok(packages)
}

#[instrument(fields(from, to), skip(packages, args))]
pub fn bump_version(args: &crate::cli::Cli, mut packages: Packages) -> Result<Packages> {
    let span = current_span!();
    info!("Bumping Version");

    if let Some(root_package) = packages.get_root_package_mut() {
        span.record("from", root_package.version().to_string());
        info!("Root package name: {}", root_package.name());
        let current_version = root_package.version_mut();
        use crate::cli::Action as BumpVer;
        let new_version = match args.action() {
            BumpVer::Patch => bump_patch(current_version),
            BumpVer::Minor => try_bump_minor(current_version, args.force_version())?,
            BumpVer::Major => try_bump_major(current_version, args.force_version())?,
            _ => unreachable!("Already sent to different function."),
        };
        span.record("to", new_version.to_string());
        info!("Completed version bump!")
    };
    Ok(packages)
}

fn bump_patch(version: &mut Version) -> Version {
    let old_version = version.clone();
    if version.pre.is_empty() {
        version.patch += 1;
    } else {
        version.pre = semver::Prerelease::EMPTY
    }

    assert!(&old_version < version);
    version.clone()
}

fn try_bump_minor(version: &mut Version, force: bool) -> Result<Version> {
    let old_version = version.clone();
    if !version.pre.is_empty() && !force {
        Err(VersionError::prerelease_not_empty(
            &old_version,
            Action::Minor,
        ))?;
    }
    version.pre = Prerelease::EMPTY;
    version.minor += 1;
    version.patch = 0;
    Ok(version.clone())
}

fn try_bump_major(version: &mut Version, force: bool) -> Result<Version> {
    let old_version = version.clone();
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
    assert!(&old_version < version);
    Ok(version.clone())
}

#[instrument(skip(packages))]
pub fn set_version(mut packages: Packages, args: &cli::Cli) -> Result<Packages> {
    let new_version = args
        .set_version
        .as_ref()
        .ok_or_else(|| miette::miette!("Called set_version with args containing version"))?;
    if let Some(root_package) = packages.get_root_package_mut() {
        let version = root_package.version_mut();
        *version = new_version.clone();
        info!("Version set.")
    };
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use semver::BuildMetadata;
    use semver::Prerelease;

    use super::*;

    macro_rules! version {
        ($maj:literal $min:literal $pat:literal) => {
            Version::new($maj, $min, $pat)
        };
        ($maj:literal $min:literal $pat:literal -$pre:ident) => {{
            let mut v = version!($maj $min $pat);
            v.pre = semver::Prerelease::new(stringify!($pre)).unwrap_or_default();
            v
        }};
        ($maj:literal $min:literal $pat:literal -$pre:ident +$build:ident) => {{
            let mut v = version!($maj $min $pat -$pre);
            v.build = semver::BuildMetadata::new(stringify!($build)).unwrap_or_default();
            v
        }};
        ($ver:literal) => {
            semver::Version::parse($ver).unwrap()
        }
    }
    #[test]
    fn test_version_macro() {
        let basic = Version::new(1, 1, 0);
        assert_eq!(basic, version!(1 1 0));

        let mut basic_with_pre = basic.clone();
        basic_with_pre.pre = semver::Prerelease::new("alpha").unwrap();
        assert_eq!(basic_with_pre, version!(1 1 0 -alpha));
        let mut basic_with_pre_and_build = basic_with_pre.clone();
        basic_with_pre_and_build.build = BuildMetadata::new("test").unwrap();
        assert_ne!(basic_with_pre, basic_with_pre_and_build);
        assert_eq!(basic_with_pre_and_build, version!(1 1 0 -alpha +test));
    }

    #[test]
    fn bump_patch_simple() {
        let mut version = version!(0 1 1);

        bump_patch(&mut version);
        assert_eq!(version, version!(0 1 2), "Bump 0.1.1 -> 0.1.2");
        bump_patch(&mut version);
        assert_eq!(version, version!(0 1 3), "Bump 0.1.2 -> 0.1.3");
    }

    #[test]
    fn bump_patch_pre() {
        let mut version = version!("0.1.1-alpha.2");

        assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
        bump_patch(&mut version);
        assert_eq!(version, version!(0 1 1));
        assert!(version > version!("0.1.1-alpha.2"));
    }

    #[test]
    fn bump_minor_simple() {
        let mut version = version!(0 1 1);

        try_bump_minor(&mut version, false).unwrap();
        assert_eq!(version, version!(0 2 0), "Bump 0.1.1 -> 0.2.0");
        try_bump_minor(&mut version, false).unwrap();
        assert_eq!(version, version!(0 3 0), "Bump 0.2.0 -> 0.3.0");
    }

    #[test]
    fn bump_minor_pre_force() {
        let mut version = version!("0.1.1-alpha.2");

        assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
        try_bump_minor(&mut version, true).unwrap();
        assert_eq!(version, version!(0 2 0), "Bump 0.1.1-alpha.2 -> 0.2.0");
        assert!(version > version!("0.1.1-alpha.2"));
    }
    #[test]
    #[should_panic]
    fn bump_minor_pre_no_force() {
        let mut version = version!("0.1.1-alpha.2");

        assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
        try_bump_minor(&mut version, false).unwrap();
    }

    #[test]
    fn bump_major_simple() -> miette::Result<()> {
        let mut version = version!(0 1 1);

        try_bump_major(&mut version, false)?;
        assert_eq!(version, version!(1 0 0), "Bump 0.1.1 -> 1.0.0");
        try_bump_major(&mut version, false)?;
        assert_eq!(version, version!(2 0 0), "Bump 1.0.0 -> 2.0.0");
        Ok(())
    }

    #[test]
    fn bump_major_pre() -> miette::Result<()> {
        let mut version = version!("0.1.1-alpha.2");

        assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
        try_bump_major(&mut version, true)?;
        assert_eq!(version, version!(2 0 0), "Bump 0.1.1 -> 2.0.0");
        assert!(version > version!("0.1.1-alpha.2"));
        Ok(())
    }

    #[test]
    #[should_panic]
    fn bump_major_pre_no_force() {
        let mut version = version!("0.1.1-alpha.2");

        assert_eq!(version.pre, Prerelease::new("alpha.2").unwrap());
        try_bump_major(&mut version, false).unwrap();
    }
}
