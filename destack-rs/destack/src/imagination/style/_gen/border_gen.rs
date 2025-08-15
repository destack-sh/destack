//! destack.imagination.style.border@2025.08.15.1

#![destack::generated(destack.imagination.style.border, file)]

#[destack::generated(BorderType, Debug, block)]
impl std::fmt::Debug for BorderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorderType::Style => write!(f, "STYLE"),
            BorderType::Solid => write!(f, "SOLID"),
            BorderType::Dashed => write!(f, "DASHED"),
            BorderType::Dotted => write!(f, "DOTTED"),
            BorderType::Double => write!(f, "DOUBLE"),
        }
    }
}