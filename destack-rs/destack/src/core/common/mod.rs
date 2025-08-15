//! destack.core.common@2025.08.15.1

#![destack::partial(core/common, file)]
#![allow(unused_imports)]

pub use crate::core::common::branch::*;
pub use crate::core::common::snapshot::*;
pub use crate::core::common::query::*;
pub use crate::core::common::_gen::*;
pub use crate::core::common::r#type::*;
pub use crate::core::common::value::*;
pub use crate::core::common::change::*;
pub use crate::core::common::icon::*;
pub use crate::core::common::text::*;
pub use crate::core::common::relation::*;

mod branch;
mod snapshot;
mod query;
mod _gen;
mod r#type;
mod value;
mod change;
mod icon;
mod text;
mod relation;

pub(crate) use crate::core::common::_gen::*;

pub(crate) use crate::core::common::_gen::*;

pub(crate) use crate::core::common::_gen::*;