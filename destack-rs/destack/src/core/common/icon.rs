//! destack.core.common.icon@2025.08.15.1

#![destack::partial(destack.core.common.icon, file)]

#[destack::generated(Icon, struct, block)]
/// An icon to be displayed in some view.
pub struct Icon {

}

#[destack::generated(IconType, enum, block)]
/// IconType
pub enum IconType {
    EMOJI = 1,
    FILE = 10,
    FILE_URL = 11
}