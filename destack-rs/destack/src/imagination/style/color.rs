//! destack.imagination.style.color@2025.08.15.1

#![destack::partial(destack.imagination.style.color, file)]

#[destack::generated(Color, struct, block)]
/// A color value.
pub struct Color {}

#[destack::generated(ColorType, enum, block)]
/// Built-in color formats.
pub enum ColorType {
    Rgb = 10,
    Hsl = 11,
    P3 = 12,
}

#[destack::generated(ColorShade, enum, block)]
/// Built-in color shades a la Tailwind.
pub enum ColorShade {
    S25 = 25,
    S50 = 50,
    S100 = 100,
    S200 = 200,
    S300 = 300,
    S400 = 400,
    S500 = 500,
    S600 = 600,
    S700 = 700,
    S800 = 800,
    S900 = 900,
    S950 = 950,
}

#[destack::generated(ColorHue, enum, block)]
/// Built-in colors a la SwiftUI or Tailwind.
pub enum ColorHue {
    Gray = 30,
    Red = 31,
    Orange = 32,
    Amber = 33,
    Yellow = 34,
    Lime = 35,
    Green = 36,
    Emerald = 37,
    Teal = 38,
    Cyan = 39,
    Sky = 40,
    Blue = 41,
    Indigo = 42,
    Violet = 43,
    Purple = 44,
    Fuchsia = 45,
    Pink = 46,
    Rose = 47,
}

#[destack::generated(ColorIntent, enum, block)]
/// Built-in color intents.
pub enum ColorIntent {
    /// A Primary intent
    Primary = 1,
    /// A Secondary intent
    Secondary = 2,
    /// A Neutral intent
    Neutral = 3,
    /// A Muted intent
    Muted = 4,
    /// A Success intent
    Success = 10,
    /// An Info intent
    Info = 11,
    /// A Warning intent
    Warning = 12,
    /// An Error intent
    Error = 13,
    /// A Critical intent
    Critical = 14,
}
