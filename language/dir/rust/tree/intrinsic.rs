use crate::StringId;

#[derive(Debug, Clone, PartialEq)]
pub enum Intrinsic {}

impl Intrinsic {
    /// Get the name of the intrinsic.
    #[inline]
    pub fn name(&self) -> StringId {
        todo!("intrinsics")
    }
}
