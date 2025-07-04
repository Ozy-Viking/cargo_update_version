use std::{path::PathBuf, str::FromStr};

use tracing::instrument;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct GitFile {
    pub mode: String,
    pub path: PathBuf,
}

impl std::fmt::Display for GitFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}

impl GitFile {
    #[instrument]
    pub fn parse(line: impl ToString + std::fmt::Debug) -> Option<GitFile> {
        let line = line.to_string();
        let (mode, path_str) = line.trim().split_once(" ")?;
        let path = PathBuf::from_str(path_str).ok()?;
        if mode.trim().is_empty() {
            None
        } else {
            Some(GitFile {
                mode: mode.trim().to_string(),
                path,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct GitFiles(Vec<GitFile>);

impl GitFiles {
    /// Creates an empty GitFiles object.
    pub fn new() -> Self {
        GitFiles(Vec::new())
    }
}

impl std::ops::Deref for GitFiles {
    type Target = Vec<GitFile>;

    fn deref(&self) -> &Vec<GitFile> {
        &self.0
    }
}

impl std::ops::DerefMut for GitFiles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<GitFile>> for GitFiles {
    fn as_ref(&self) -> &Vec<GitFile> {
        &self.0
    }
}

impl AsMut<Vec<GitFile>> for GitFiles {
    fn as_mut(&mut self) -> &mut Vec<GitFile> {
        &mut self.0
    }
}

impl std::fmt::Display for GitFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for file in self.0.iter() {
            writeln!(f, "{}", file)?;
        }
        Ok(())
    }
}

impl GitFiles {
    #[instrument]
    /// Mutates the git output for `git status --short` to a vec of files.
    /// Returns None if [Vec] is empty.
    pub fn parse(lines: String) -> Option<Self> {
        let lines = lines.lines();
        let mut ret = Vec::new();
        for line in lines {
            if let Some(gfile) = GitFile::parse(line) {
                ret.push(gfile);
            }
        }
        if ret.is_empty() {
            None
        } else {
            Some(GitFiles(ret))
        }
    }
}
