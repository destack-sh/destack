//! destack.imagination.style.font@2025.08.15.1

#![destack::generated(destack.imagination.style.font, file)]

#[destack::generated(FontType, Debug, block)]
impl std::fmt::Debug for FontType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontType::Serif => write!(f, "SERIF"),
            FontType::Sans => write!(f, "SANS"),
            FontType::Mono => write!(f, "MONO"),
        }
    }
}

#[destack::generated(FontWeight, Debug, block)]
impl std::fmt::Debug for FontWeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontWeight::Thin => write!(f, "THIN"),
            FontWeight::ExtraLight => write!(f, "EXTRA_LIGHT"),
            FontWeight::Light => write!(f, "LIGHT"),
            FontWeight::Normal => write!(f, "NORMAL"),
            FontWeight::Medium => write!(f, "MEDIUM"),
            FontWeight::SemiBold => write!(f, "SEMI_BOLD"),
            FontWeight::Bold => write!(f, "BOLD"),
            FontWeight::ExtraBold => write!(f, "EXTRA_BOLD"),
            FontWeight::Black => write!(f, "BLACK"),
        }
    }
}

#[destack::generated(FontSize, Debug, block)]
impl std::fmt::Debug for FontSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontSize::Xs => write!(f, "XS"),
            FontSize::Sm => write!(f, "SM"),
            FontSize::Base => write!(f, "BASE"),
            FontSize::Lg => write!(f, "LG"),
            FontSize::Xl => write!(f, "XL"),
            FontSize::Xl2 => write!(f, "XL2"),
            FontSize::Xl3 => write!(f, "XL3"),
            FontSize::Xl4 => write!(f, "XL4"),
            FontSize::Xl5 => write!(f, "XL5"),
            FontSize::Xl6 => write!(f, "XL6"),
            FontSize::Xl7 => write!(f, "XL7"),
        }
    }
}

#[destack::generated(TextAlign, Debug, block)]
impl std::fmt::Debug for TextAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextAlign::Left => write!(f, "LEFT"),
            TextAlign::Center => write!(f, "CENTER"),
            TextAlign::Right => write!(f, "RIGHT"),
            TextAlign::Justify => write!(f, "JUSTIFY"),
        }
    }
}

#[destack::generated(TextDecoration, Debug, block)]
impl std::fmt::Debug for TextDecoration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextDecoration::None => write!(f, "NONE"),
            TextDecoration::Underline => write!(f, "UNDERLINE"),
            TextDecoration::Strikethrough => write!(f, "STRIKETHROUGH"),
        }
    }
}

#[destack::generated(TextTransform, Debug, block)]
impl std::fmt::Debug for TextTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextTransform::None => write!(f, "NONE"),
            TextTransform::Uppercase => write!(f, "UPPERCASE"),
            TextTransform::Lowercase => write!(f, "LOWERCASE"),
            TextTransform::Capitalize => write!(f, "CAPITALIZE"),
        }
    }
}
