use crate::PackageName;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[diagnostic(url(docsrs))]
pub enum PackageError {
    #[error("Package ({0}) not found in Cargo.toml")]
    #[diagnostic(code(PackageError::PackageNameNotFound))]
    PackageNameNotFound(PackageName),
    #[error("Package name not provided")]
    #[diagnostic(code(PackageError::PackageNameNotProvided))]
    PackageNameNotProvided,
}
