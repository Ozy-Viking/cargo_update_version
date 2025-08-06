use std::{borrow::Borrow, fmt::Display, ops::DerefMut};

/// Newtype around Package Name.
///
/// `workspace.package` for the workspace package as '.' is an invalid char for a package name.
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub struct PackageName(pub String);

impl Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<T> From<T> for PackageName
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl std::ops::Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PackageName {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Borrow<std::string::String> for PackageName {
    fn borrow(&self) -> &std::string::String {
        &self.0
    }
}
impl Borrow<std::string::String> for &PackageName {
    fn borrow(&self) -> &std::string::String {
        &self.0
    }
}
impl AsRef<str> for PackageName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsMut<str> for PackageName {
    fn as_mut(&mut self) -> &mut str {
        &mut self.0
    }
}

impl PartialEq<String> for PackageName {
    fn eq(&self, other: &String) -> bool {
        &self.0 == other
    }
}

impl PartialEq<PackageName> for String {
    fn eq(&self, other: &PackageName) -> bool {
        other.eq(self)
    }
}

impl PackageName {
    pub fn is_workspace_package(&self) -> bool {
        self.0 == PackageName::workspace_package()
    }

    /// [`PackageName("workspace.package")`]
    pub fn workspace_package() -> PackageName {
        PackageName("workspace.package".into())
    }
}
