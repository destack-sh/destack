//! destack.imagination.style.font@2025.08.15.1

#![destack::partial(destack.imagination.style.font, file)]

#[destack::generated(Font, struct, block)]
/// A font value.
pub struct Font {

}

#[destack::generated(FontType, enum, block)]
/// FontType
pub enum FontType {
    /// A serif font
    SERIF = 10,
    /// A sans-serif font
    SANS = 11,
    /// A monospace font
    MONO = 12
}

#[destack::generated(FontWeight, enum, block)]
/// FontWeight
pub enum FontWeight {
    /// A thin font weight
    THIN = 100,
    /// An extra light font weight
    EXTRA_LIGHT = 200,
    /// A light font weight
    LIGHT = 300,
    /// A normal font weight
    NORMAL = 400,
    /// A medium font weight
    MEDIUM = 500,
    /// A semi-bold font weight
    SEMI_BOLD = 600,
    /// A bold font weight
    BOLD = 700,
    /// An extra bold font weight
    EXTRA_BOLD = 800,
    /// A black font weight
    BLACK = 900
}

#[destack::generated(FontSize, enum, block)]
/// FontSize
pub enum FontSize {
    /// An XS font size
    XS = 12,
    /// An SM font size
    SM = 14,
    /// A base font size
    BASE = 16,
    /// An LG font size
    LG = 18,
    /// An XL font size
    XL = 20,
    /// An XL2 font size
    XL2 = 24,
    /// An XL3 font size
    XL3 = 30,
    /// An XL4 font size
    XL4 = 36,
    /// An XL5 font size
    XL5 = 48,
    /// An XL6 font size
    XL6 = 60,
    /// An XL7 font size
    XL7 = 72
}

#[destack::generated(TextAlign, enum, block)]
/// TextAlign
pub enum TextAlign {
    /// A left text alignment
    LEFT = 1,
    /// A center text alignment
    CENTER = 2,
    /// A right text alignment
    RIGHT = 3,
    /// A justify text alignment
    JUSTIFY = 4
}

#[destack::generated(TextDecoration, enum, block)]
/// TextDecoration
pub enum TextDecoration {
    /// No text decoration
    NONE = 1,
    /// An underline text decoration
    UNDERLINE = 2,
    /// A strikethrough text decoration
    STRIKETHROUGH = 3
}

#[destack::generated(TextTransform, enum, block)]
/// TextTransform
pub enum TextTransform {
    /// No text transform
    NONE = 1,
    /// An uppercase text transform
    UPPERCASE = 2,
    /// A lowercase text transform
    LOWERCASE = 3,
    /// A capitalize text transform
    CAPITALIZE = 4
}