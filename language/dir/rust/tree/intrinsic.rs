use crate::StringId;

#[derive(Debug, Clone, PartialEq)]
pub enum Intrinsic {}

impl Intrinsic {
    pub fn name(&self) -> StringId {
        todo!("intrinsics")
    }
}
