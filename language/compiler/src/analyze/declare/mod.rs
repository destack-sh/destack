mod attribute;
mod collect;
mod r#override;
mod process;
mod r#type;

pub(crate) use r#type::StaticConstantResolutionMode;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use r#type::TypeMemberResolution;
