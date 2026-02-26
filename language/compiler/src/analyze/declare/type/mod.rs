mod cache;
mod expression;
mod resolve;
mod r#static;

pub(crate) use r#static::StaticConstantResolutionMode;

#[cfg(test)]
pub(crate) use resolve::TypeMemberResolution;
