//! destack.imagination.style.color@2025.08.15.1

#![destack::partial(destack.imagination.style.color, file)]

#[destack::generated(Color, struct, block)]
/// A color value.
pub struct Color {

}

#[destack::generated(ColorType, enum, block)]
/// Built-in color formats.
pub enum ColorType {
    RGB = 10,
    HSL = 11,
    P3 = 12
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
    S950 = 950
}

#[destack::generated(ColorHue, enum, block)]
/// Built-in colors a la SwiftUI or Tailwind.
pub enum ColorHue {
    GRAY = 30,
    RED = 31,
    ORANGE = 32,
    AMBER = 33,
    YELLOW = 34,
    LIME = 35,
    GREEN = 36,
    EMERALD = 37,
    TEAL = 38,
    CYAN = 39,
    SKY = 40,
    BLUE = 41,
    INDIGO = 42,
    VIOLET = 43,
    PURPLE = 44,
    FUCHSIA = 45,
    PINK = 46,
    ROSE = 47
}

#[destack::generated(ColorIntent, enum, block)]
/// Built-in color intents.
pub enum ColorIntent {
    /// A Primary intent
    PRIMARY = 1,
    /// A Secondary intent
    SECONDARY = 2,
    /// A Neutral intent
    NEUTRAL = 3,
    /// A Muted intent
    MUTED = 4,
    /// A Success intent
    SUCCESS = 10,
    /// An Info intent
    INFO = 11,
    /// A Warning intent
    WARNING = 12,
    /// An Error intent
    ERROR = 13,
    /// A Critical intent
    CRITICAL = 14
}