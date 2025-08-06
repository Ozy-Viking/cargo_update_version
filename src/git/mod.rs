pub(crate) mod branch;
#[allow(clippy::module_inception)]
pub(crate) mod git;
pub(crate) mod git_file;

pub use branch::Branch;
pub use git::Git;
pub use git::GitBuilder;
pub use git::NoRootDirSet;
pub use git::Stash;
pub use git_file::GitFile;
pub use git_file::GitFiles;
