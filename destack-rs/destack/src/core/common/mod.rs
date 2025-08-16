//! destack.core.common@2025.08.15.1

#![destack::partial(destack.core.common, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::core::common::change::*;
pub use crate::core::common::icon::*;
pub use crate::core::common::query::*;
pub use crate::core::common::relation::*;
pub use crate::core::common::text::*;
pub use crate::core::common::r#type::*;
pub use crate::core::common::value::*;

pub mod _gen;
pub mod change;
pub mod icon;
pub mod query;
pub mod relation;
pub mod text;
pub mod r#type;
pub mod value;
