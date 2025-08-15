//! destack.core.common.text@2025.08.15.1

#![destack::generated(destack.core.common.text, file)]

use crate::{TextSpanType, TextStyleFlag};

#[destack::generated(TextSpanType, Debug, block)]
impl std::fmt::Debug for TextSpanType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextSpanType::Text => write!(f, "TEXT"),
            TextSpanType::HardBreak => write!(f, "HARD_BREAK"),
            TextSpanType::Node => write!(f, "NODE"),
            TextSpanType::Link => write!(f, "LINK"),
            TextSpanType::Equation => write!(f, "EQUATION"),
        }
    }
}

#[destack::generated(TextStyleFlag, Debug, block)]
impl std::fmt::Debug for TextStyleFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextStyleFlag::Default => write!(f, "DEFAULT"),
            TextStyleFlag::Bold => write!(f, "BOLD"),
            TextStyleFlag::Italic => write!(f, "ITALIC"),
            TextStyleFlag::Strikethrough => write!(f, "STRIKETHROUGH"),
            TextStyleFlag::Underline => write!(f, "UNDERLINE"),
            TextStyleFlag::Code => write!(f, "CODE"),
        }
    }
}
