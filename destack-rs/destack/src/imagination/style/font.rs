//! destack.imagination.style.font

#![destack::partial(destack.imagination.style.font, file)]

use crate::{Fill, Length};

#[destack::generated(Font, -, block)]
/// A font value.
pub struct Font {
    pub r#type: FontType,
    pub template: Option<i64>,
    pub weight: Option<FontWeight>,
    pub fill: Option<Fill>,
    pub size: Option<FontSize>,
    pub align: Option<TextAlign>,
    pub line_height: Option<Length>,
    pub letter_spacing: Option<Length>,
    pub decoration: Option<TextDecoration>,
    pub transform: Option<TextTransform>,
}

#[destack::generated(FontType, -, block)]
/// FontType
pub enum FontType {
    /// A serif font
    Serif = 10,
    /// A sans-serif font
    Sans = 11,
    /// A monospace font
    Mono = 12,
}

#[destack::generated(FontWeight, -, block)]
/// FontWeight
pub enum FontWeight {
    /// A thin font weight
    Thin = 100,
    /// An extra light font weight
    ExtraLight = 200,
    /// A light font weight
    Light = 300,
    /// A normal font weight
    Normal = 400,
    /// A medium font weight
    Medium = 500,
    /// A semi-bold font weight
    SemiBold = 600,
    /// A bold font weight
    Bold = 700,
    /// An extra bold font weight
    ExtraBold = 800,
    /// A black font weight
    Black = 900,
}

#[destack::generated(FontSize, -, block)]
/// FontSize
pub enum FontSize {
    /// An XS font size
    Xs = 12,
    /// An SM font size
    Sm = 14,
    /// A base font size
    Base = 16,
    /// An LG font size
    Lg = 18,
    /// An XL font size
    Xl = 20,
    /// An XL2 font size
    Xl2 = 24,
    /// An XL3 font size
    Xl3 = 30,
    /// An XL4 font size
    Xl4 = 36,
    /// An XL5 font size
    Xl5 = 48,
    /// An XL6 font size
    Xl6 = 60,
    /// An XL7 font size
    Xl7 = 72,
}

#[destack::generated(TextAlign, -, block)]
/// TextAlign
pub enum TextAlign {
    /// A left text alignment
    Left = 1,
    /// A center text alignment
    Center = 2,
    /// A right text alignment
    Right = 3,
    /// A justify text alignment
    Justify = 4,
}

#[destack::generated(TextDecoration, -, block)]
/// TextDecoration
pub enum TextDecoration {
    /// No text decoration
    None = 1,
    /// An underline text decoration
    Underline = 2,
    /// A strikethrough text decoration
    Strikethrough = 3,
}

#[destack::generated(TextTransform, -, block)]
/// TextTransform
pub enum TextTransform {
    /// No text transform
    None = 1,
    /// An uppercase text transform
    Uppercase = 2,
    /// A lowercase text transform
    Lowercase = 3,
    /// A capitalize text transform
    Capitalize = 4,
}
