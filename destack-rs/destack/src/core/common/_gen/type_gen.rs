//! destack.core.common.type@2025.08.15.1

#![destack::generated(destack.core.common.type, file)]

use crate::{CollectionConstraint, NumberConstraint, StringConstraint, Type};

#[destack::generated(Type, Debug, block)]
impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Type")
    }
}

#[destack::generated(NumberConstraint, Debug, block)]
impl std::fmt::Debug for NumberConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NumberConstraint")
    }
}

#[destack::generated(StringConstraint, Debug, block)]
impl std::fmt::Debug for StringConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StringConstraint")
    }
}

#[destack::generated(CollectionConstraint, Debug, block)]
impl std::fmt::Debug for CollectionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CollectionConstraint")
    }
}
