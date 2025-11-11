use crate::StringId;

#[derive(Debug, Clone, PartialEq)]
pub enum Intrinsic {}

impl Intrinsic {
    pub fn name(&self) -> StringId {
        panic!("NOTE #Incomplete: intrinsics")
    }
}
