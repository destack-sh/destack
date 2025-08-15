//! destack.core.common.icon@2025.08.15.1

#![destack::partial(destack.core.common.icon, file)]

use crate::Color;
use crate::IconType;

#[destack::generated(Icon, , block)]
/// An icon to be displayed in some view.
pub struct Icon {
    r#type: IconType,
    emoji: Option<String>,
    fa_name: Option<String>,
    vsc_name: Option<String>,
    file: Option<i64 /* TODO */>,
    file_url: Option<String>,
    color: Option<Color>,
}

#[destack::generated(IconType, , block)]
/// IconType
pub enum IconType {
    Emoji = 1,
    File = 10,
    FileUrl = 11,
}
