mod argument;
mod layout;
mod render;

pub(in crate::format) use self::argument::argument_satisfies_static_seam_comment_source;
pub(crate) use self::layout::*;
pub(crate) use self::render::*;
