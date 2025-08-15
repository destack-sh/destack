//! destack.imagination.animation.easing@2025.08.15.1

#![destack::generated(destack.imagination.animation.easing, file)]

#[destack::generated(Easing, Debug, block)]
impl std::fmt::Debug for Easing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Easing::Linear => write!(f, "LINEAR"),
            Easing::EaseInQuad => write!(f, "EASE_IN_QUAD"),
            Easing::EaseOutQuad => write!(f, "EASE_OUT_QUAD"),
            Easing::EaseInOutQuad => write!(f, "EASE_IN_OUT_QUAD"),
            Easing::EaseInCubic => write!(f, "EASE_IN_CUBIC"),
            Easing::EaseOutCubic => write!(f, "EASE_OUT_CUBIC"),
            Easing::EaseInOutCubic => write!(f, "EASE_IN_OUT_CUBIC"),
            Easing::EaseInQuart => write!(f, "EASE_IN_QUART"),
            Easing::EaseOutQuart => write!(f, "EASE_OUT_QUART"),
            Easing::EaseInOutQuart => write!(f, "EASE_IN_OUT_QUART"),
            Easing::EaseInQuint => write!(f, "EASE_IN_QUINT"),
            Easing::EaseOutQuint => write!(f, "EASE_OUT_QUINT"),
            Easing::EaseInOutQuint => write!(f, "EASE_IN_OUT_QUINT"),
            Easing::EaseInSine => write!(f, "EASE_IN_SINE"),
            Easing::EaseOutSine => write!(f, "EASE_OUT_SINE"),
            Easing::EaseInOutSine => write!(f, "EASE_IN_OUT_SINE"),
            Easing::EaseInExpo => write!(f, "EASE_IN_EXPO"),
            Easing::EaseOutExpo => write!(f, "EASE_OUT_EXPO"),
            Easing::EaseInOutExpo => write!(f, "EASE_IN_OUT_EXPO"),
            Easing::EasePen => write!(f, "EASE_PEN"),
        }
    }
}