use core::fmt;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SiteId(pub usize);
impl Deref for SiteId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<usize> for SiteId {
    fn from(id: usize) -> Self {
        SiteId(id)
    }
}
impl From<SiteId> for usize {
    fn from(id: SiteId) -> Self {
        id.0
    }
}
impl fmt::Display for SiteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
