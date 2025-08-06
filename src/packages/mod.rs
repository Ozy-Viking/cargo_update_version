mod package_name;
pub use package_name::PackageName;

mod package;
pub use package::Package;

#[allow(clippy::module_inception)]
mod packages;
pub use packages::Packages;

mod error;
pub use error::PackageError;
