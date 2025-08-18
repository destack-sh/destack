//! destack.imagination.style.color

#![destack::generated(destack.imagination.style.color, file)]

use crate::{Color, ColorHue, ColorIntent, ColorShade, ColorType};

#[destack::generated(Color, Debug, block)]
impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Color")
    }
}

#[destack::generated(ColorType, Debug, block)]
impl std::fmt::Debug for ColorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorType::Rgb => write!(f, "RGB"),
            ColorType::Hsl => write!(f, "HSL"),
            ColorType::P3 => write!(f, "P3"),
        }
    }
}

#[destack::generated(ColorShade, Debug, block)]
impl std::fmt::Debug for ColorShade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorShade::S25 => write!(f, "S25"),
            ColorShade::S50 => write!(f, "S50"),
            ColorShade::S100 => write!(f, "S100"),
            ColorShade::S200 => write!(f, "S200"),
            ColorShade::S300 => write!(f, "S300"),
            ColorShade::S400 => write!(f, "S400"),
            ColorShade::S500 => write!(f, "S500"),
            ColorShade::S600 => write!(f, "S600"),
            ColorShade::S700 => write!(f, "S700"),
            ColorShade::S800 => write!(f, "S800"),
            ColorShade::S900 => write!(f, "S900"),
            ColorShade::S950 => write!(f, "S950"),
        }
    }
}

#[destack::generated(ColorHue, Debug, block)]
impl std::fmt::Debug for ColorHue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorHue::Gray => write!(f, "GRAY"),
            ColorHue::Red => write!(f, "RED"),
            ColorHue::Orange => write!(f, "ORANGE"),
            ColorHue::Amber => write!(f, "AMBER"),
            ColorHue::Yellow => write!(f, "YELLOW"),
            ColorHue::Lime => write!(f, "LIME"),
            ColorHue::Green => write!(f, "GREEN"),
            ColorHue::Emerald => write!(f, "EMERALD"),
            ColorHue::Teal => write!(f, "TEAL"),
            ColorHue::Cyan => write!(f, "CYAN"),
            ColorHue::Sky => write!(f, "SKY"),
            ColorHue::Blue => write!(f, "BLUE"),
            ColorHue::Indigo => write!(f, "INDIGO"),
            ColorHue::Violet => write!(f, "VIOLET"),
            ColorHue::Purple => write!(f, "PURPLE"),
            ColorHue::Fuchsia => write!(f, "FUCHSIA"),
            ColorHue::Pink => write!(f, "PINK"),
            ColorHue::Rose => write!(f, "ROSE"),
        }
    }
}

#[destack::generated(ColorIntent, Debug, block)]
impl std::fmt::Debug for ColorIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorIntent::Primary => write!(f, "PRIMARY"),
            ColorIntent::Secondary => write!(f, "SECONDARY"),
            ColorIntent::Neutral => write!(f, "NEUTRAL"),
            ColorIntent::Muted => write!(f, "MUTED"),
            ColorIntent::Success => write!(f, "SUCCESS"),
            ColorIntent::Info => write!(f, "INFO"),
            ColorIntent::Warning => write!(f, "WARNING"),
            ColorIntent::Error => write!(f, "ERROR"),
            ColorIntent::Critical => write!(f, "CRITICAL"),
        }
    }
}
