//! destack.core.common.icon@2025.08.15.1

#![destack::partial(destack.core.common.icon, file)]

use crate::Color;

#[destack::generated(Icon, struct, block)]
/// An icon to be displayed in some view.
pub struct Icon {
    r#type: IconType,
    emoji: String,
    fa_name: String,
    vsc_name: String,
    file: i64, /* TODO */
    file_url: String,
    color: Color,
}

#[destack::generated(IconType, enum, block)]
/// IconType
pub enum IconType {
    Emoji = 1,
    File = 10,
    FileUrl = 11,
}
