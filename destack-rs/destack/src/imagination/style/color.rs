//! destack.imagination.style.color

#![destack::partial(destack.imagination.style.color, file)]

#[destack::generated(Color, -, block)]
/// A color value.
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[destack::generated(ColorType, -, block)]
/// Built-in color formats.
pub enum ColorType {
    Rgb = 10,
    Hsl = 11,
    P3 = 12,
}

#[destack::generated(ColorShade, -, block)]
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

#[destack::generated(ColorHue, -, block)]
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

#[destack::generated(ColorIntent, -, block)]
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

#[destack::partial(destack.imagination.style.color, hex_to_rgb, block)]
/// Convert hex color string to linear-space RGB floats with optional alpha.
pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32, f32) {
    let hex = hex.trim_start_matches('#');

    // parse hex digits efficiently
    let parse_hex_byte = |s: &str| -> Option<u8> { u8::from_str_radix(s, 16).ok() };

    let (r, g, b, a) = match hex.len() {
        // RGB shorthand (#F0A -> #FF00AA)
        3 => {
            let chars: Vec<char> = hex.chars().collect();
            if chars.len() != 3 {
                return (0.0, 0.0, 0.0, 1.0);
            }
            let r = parse_hex_byte(&format!("{}{}", chars[0], chars[0])).unwrap_or(0);
            let g = parse_hex_byte(&format!("{}{}", chars[1], chars[1])).unwrap_or(0);
            let b = parse_hex_byte(&format!("{}{}", chars[2], chars[2])).unwrap_or(0);
            (r, g, b, 255)
        }
        // RGBA shorthand (#F0A8 -> #FF00AA88)
        4 => {
            let chars: Vec<char> = hex.chars().collect();
            if chars.len() != 4 {
                return (0.0, 0.0, 0.0, 1.0);
            }
            let r = parse_hex_byte(&format!("{}{}", chars[0], chars[0])).unwrap_or(0);
            let g = parse_hex_byte(&format!("{}{}", chars[1], chars[1])).unwrap_or(0);
            let b = parse_hex_byte(&format!("{}{}", chars[2], chars[2])).unwrap_or(0);
            let a = parse_hex_byte(&format!("{}{}", chars[3], chars[3])).unwrap_or(255);
            (r, g, b, a)
        }
        // RGB full (#FF00AA)
        6 => {
            let r = parse_hex_byte(&hex[0..2]).unwrap_or(0);
            let g = parse_hex_byte(&hex[2..4]).unwrap_or(0);
            let b = parse_hex_byte(&hex[4..6]).unwrap_or(0);
            (r, g, b, 255)
        }
        // RGBA full (#FF00AA88)
        8 => {
            let r = parse_hex_byte(&hex[0..2]).unwrap_or(0);
            let g = parse_hex_byte(&hex[2..4]).unwrap_or(0);
            let b = parse_hex_byte(&hex[4..6]).unwrap_or(0);
            let a = parse_hex_byte(&hex[6..8]).unwrap_or(255);
            (r, g, b, a)
        }
        _ => return (0.0, 0.0, 0.0, 1.0),
    };

    // convert from sRGB gamma space to linear space
    let srgb_to_linear = |c: u8| -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    (
        srgb_to_linear(r),
        srgb_to_linear(g),
        srgb_to_linear(b),
        a as f32 / 255.0,
    )
}

#[destack::partial(destack.imagination.style.color, rgb_to_hex, block)]
/// Convert 8-bit sRGB values to hex color string.
pub fn rgb_to_hex(r: u32, g: u32, b: u32, a: Option<u32>) -> String {
    // clamp values to 0-255 range
    let r = (r.min(255)) as u8;
    let g = (g.min(255)) as u8;
    let b = (b.min(255)) as u8;

    match a {
        Some(a) if a < 255 => {
            let a = (a.min(255)) as u8;
            format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
        }
        _ => format!("#{r:02x}{g:02x}{b:02x}"),
    }
}

#[destack::partial(destack.imagination.style.color, rgb_to_hsl, block)]
/// Convert 8-bit sRGB values to HSL color space.
pub fn rgb_to_hsl(r: u32, g: u32, b: u32) -> (f32, f32, f32) {
    // normalize to 0.0-1.0 range
    let r = (r.min(255) as f32) / 255.0;
    let g = (g.min(255) as f32) / 255.0;
    let b = (b.min(255) as f32) / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // lightness
    let l = (max + min) / 2.0;

    if delta == 0.0 {
        // achromatic (gray)
        return (0.0, 0.0, l);
    }

    // saturation
    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    // hue
    let h = if (max - r).abs() < f32::EPSILON {
        ((g - b) / delta + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    (h, s, l)
}

#[destack::partial(destack.imagination.style.color, hsl_to_rgb, block)]
/// Convert HSL values to linear-space RGB floats.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    // clamp inputs
    let h = h.clamp(0.0, 1.0);
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s == 0.0 {
        // achromatic - convert gray to linear space
        let srgb_to_linear = |c: f32| -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        let linear = srgb_to_linear(l);
        return (linear, linear, linear);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };

    let p = 2.0 * l - q;

    let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }

        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    // convert from sRGB gamma space to linear space
    let srgb_to_linear = |c: f32| -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b))
}

#[destack::partial(destack.imagination.style.color, rgb_to_p3, block)]
/// Convert gamma-encoded sRGB to gamma-encoded Display-P3.
pub fn rgb_to_p3(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // clamp inputs
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);

    // convert to linear RGB
    let srgb_to_linear = |c: f32| -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r_linear = srgb_to_linear(r);
    let g_linear = srgb_to_linear(g);
    let b_linear = srgb_to_linear(b);

    // matrix transformation from linear sRGB to linear Display-P3
    // based on the standard D65 white point conversion
    let r_p3_linear = 0.8224619 * r_linear + 0.1775381 * g_linear + 0.0 * b_linear;
    let g_p3_linear = 0.0331942 * r_linear + 0.9668058 * g_linear + 0.0 * b_linear;
    let b_p3_linear = 0.0170827 * r_linear + 0.0723975 * g_linear + 0.9105198 * b_linear;

    // convert back to gamma-encoded P3
    let linear_to_p3_gamma = |c: f32| -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    };

    (
        linear_to_p3_gamma(r_p3_linear).clamp(0.0, 1.0),
        linear_to_p3_gamma(g_p3_linear).clamp(0.0, 1.0),
        linear_to_p3_gamma(b_p3_linear).clamp(0.0, 1.0),
    )
}

#[destack::partial(destack.imagination.style.color, p3_to_rgb, block)]
/// Convert gamma-encoded Display-P3 to gamma-encoded sRGB.
pub fn p3_to_rgb(rp3: f32, gp3: f32, bp3: f32) -> (f32, f32, f32) {
    // clamp inputs
    let rp3 = rp3.clamp(0.0, 1.0);
    let gp3 = gp3.clamp(0.0, 1.0);
    let bp3 = bp3.clamp(0.0, 1.0);

    // convert to linear P3
    let p3_gamma_to_linear = |c: f32| -> f32 {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };

    let r_p3_linear = p3_gamma_to_linear(rp3);
    let g_p3_linear = p3_gamma_to_linear(gp3);
    let b_p3_linear = p3_gamma_to_linear(bp3);

    // inverse matrix transformation from linear Display-P3 to linear sRGB
    let r_linear = 1.2249402 * r_p3_linear - 0.2249402 * g_p3_linear - 0.0 * b_p3_linear;
    let g_linear = -0.042057 * r_p3_linear + 1.042057 * g_p3_linear - 0.0 * b_p3_linear;
    let b_linear = -0.0196377 * r_p3_linear - 0.0786361 * g_p3_linear + 1.0982738 * b_p3_linear;

    // convert back to gamma-encoded sRGB
    let linear_to_srgb = |c: f32| -> f32 {
        if c <= 0.0031308 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }
    };

    (
        linear_to_srgb(r_linear).clamp(0.0, 1.0),
        linear_to_srgb(g_linear).clamp(0.0, 1.0),
        linear_to_srgb(b_linear).clamp(0.0, 1.0),
    )
}

#[destack::partial(destack.imagination.style.color, hsl_to_p3, block)]
/// Convert HSL to gamma-encoded Display-P3.
pub fn hsl_to_p3(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    // first convert HSL to sRGB (but we need gamma-encoded, not linear)
    // so we'll do a modified version that gives us gamma-encoded RGB
    let h = h.clamp(0.0, 1.0);
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    let (r, g, b) = if s == 0.0 {
        // achromatic
        (l, l, l)
    } else {
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };

        let p = 2.0 * l - q;

        let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }

            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };

        (
            hue_to_rgb(p, q, h + 1.0 / 3.0),
            hue_to_rgb(p, q, h),
            hue_to_rgb(p, q, h - 1.0 / 3.0),
        )
    };

    // now convert gamma-encoded sRGB to gamma-encoded P3
    rgb_to_p3(r, g, b)
}

#[destack::partial(destack.imagination.style.color, p3_to_hsl, block)]
/// Convert gamma-encoded Display-P3 to HSL.
pub fn p3_to_hsl(rp3: f32, gp3: f32, bp3: f32) -> (f32, f32, f32) {
    // first convert P3 to sRGB
    let (r, g, b) = p3_to_rgb(rp3, gp3, bp3);

    // then convert sRGB to HSL (treating as 0-1 range values)
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    // lightness
    let l = (max + min) / 2.0;

    if delta == 0.0 {
        // achromatic (gray)
        return (0.0, 0.0, l);
    }

    // saturation
    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    // hue
    let h = if (max - r).abs() < f32::EPSILON {
        ((g - b) / delta + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < f32::EPSILON {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };

    (h, s, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    fn approx_eq_tuple3(a: (f32, f32, f32), b: (f32, f32, f32), epsilon: f32) -> bool {
        approx_eq(a.0, b.0, epsilon) && approx_eq(a.1, b.1, epsilon) && approx_eq(a.2, b.2, epsilon)
    }

    fn approx_eq_tuple4(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32), epsilon: f32) -> bool {
        approx_eq(a.0, b.0, epsilon)
            && approx_eq(a.1, b.1, epsilon)
            && approx_eq(a.2, b.2, epsilon)
            && approx_eq(a.3, b.3, epsilon)
    }

    #[test]
    fn test_hex_to_rgb() {
        // test standard 6-digit hex
        let (r, g, b, a) = hex_to_rgb("#FF0000");
        assert!(approx_eq(r, 1.0, 0.01));
        assert!(approx_eq(g, 0.0, 0.01));
        assert!(approx_eq(b, 0.0, 0.01));
        assert!(approx_eq(a, 1.0, 0.01));

        // test 3-digit shorthand
        let (r, g, b, a) = hex_to_rgb("#F0A");
        let expected = hex_to_rgb("#FF00AA");
        assert!(approx_eq_tuple4((r, g, b, a), expected, 0.01));

        // test 8-digit with alpha
        let (r, g, b, a) = hex_to_rgb("#FF00FF80");
        assert!(approx_eq(r, 1.0, 0.01));
        assert!(approx_eq(g, 0.0, 0.01));
        assert!(approx_eq(b, 1.0, 0.01));
        assert!(approx_eq(a, 0.502, 0.01)); // 0x80 / 255 ≈ 0.502

        // test 4-digit shorthand with alpha
        let (r, g, b, a) = hex_to_rgb("#F0A8");
        let expected = hex_to_rgb("#FF00AA88");
        assert!(approx_eq_tuple4((r, g, b, a), expected, 0.01));

        // test without # prefix
        let result = hex_to_rgb("00FF00");
        assert!(approx_eq(result.1, 1.0, 0.01)); // green (255) in linear space is 1.0

        // test invalid hex
        let (r, g, b, a) = hex_to_rgb("#GGGGGG");
        assert!(approx_eq_tuple4((r, g, b, a), (0.0, 0.0, 0.0, 1.0), 0.01));

        // test black and white
        let black = hex_to_rgb("#000000");
        assert!(approx_eq_tuple4(black, (0.0, 0.0, 0.0, 1.0), 0.01));

        let white = hex_to_rgb("#FFFFFF");
        assert!(approx_eq_tuple4(white, (1.0, 1.0, 1.0, 1.0), 0.01));
    }

    #[test]
    fn test_rgb_to_hex() {
        // test standard RGB
        assert_eq!(rgb_to_hex(255, 0, 0, None), "#ff0000");
        assert_eq!(rgb_to_hex(0, 255, 0, None), "#00ff00");
        assert_eq!(rgb_to_hex(0, 0, 255, None), "#0000ff");

        // test with alpha
        assert_eq!(rgb_to_hex(255, 0, 255, Some(128)), "#ff00ff80");
        assert_eq!(rgb_to_hex(255, 255, 255, Some(255)), "#ffffff");

        // test clamping
        assert_eq!(rgb_to_hex(300, 300, 300, None), "#ffffff");
        assert_eq!(rgb_to_hex(300, 0, 0, Some(300)), "#ff0000");

        // test black and gray
        assert_eq!(rgb_to_hex(0, 0, 0, None), "#000000");
        assert_eq!(rgb_to_hex(128, 128, 128, None), "#808080");

        // test alpha edge cases
        assert_eq!(rgb_to_hex(255, 0, 0, Some(0)), "#ff000000");
        assert_eq!(rgb_to_hex(255, 0, 0, Some(254)), "#ff0000fe");
    }

    #[test]
    fn test_rgb_to_hsl() {
        // test pure red
        let (h, s, l) = rgb_to_hsl(255, 0, 0);
        assert!(approx_eq(h, 0.0, 0.01));
        assert!(approx_eq(s, 1.0, 0.01));
        assert!(approx_eq(l, 0.5, 0.01));

        // test pure green
        let (h, s, l) = rgb_to_hsl(0, 255, 0);
        assert!(approx_eq(h, 0.333, 0.01));
        assert!(approx_eq(s, 1.0, 0.01));
        assert!(approx_eq(l, 0.5, 0.01));

        // test pure blue
        let (h, s, l) = rgb_to_hsl(0, 0, 255);
        assert!(approx_eq(h, 0.667, 0.01));
        assert!(approx_eq(s, 1.0, 0.01));
        assert!(approx_eq(l, 0.5, 0.01));

        // test gray (no saturation)
        let (h, s, l) = rgb_to_hsl(128, 128, 128);
        assert!(approx_eq(h, 0.0, 0.01));
        assert!(approx_eq(s, 0.0, 0.01));
        assert!(approx_eq(l, 0.502, 0.01));

        // test white
        let (h, s, l) = rgb_to_hsl(255, 255, 255);
        assert!(approx_eq(h, 0.0, 0.01));
        assert!(approx_eq(s, 0.0, 0.01));
        assert!(approx_eq(l, 1.0, 0.01));

        // test black
        let (h, s, l) = rgb_to_hsl(0, 0, 0);
        assert!(approx_eq(h, 0.0, 0.01));
        assert!(approx_eq(s, 0.0, 0.01));
        assert!(approx_eq(l, 0.0, 0.01));

        // test clamping
        let (h, s, l) = rgb_to_hsl(300, 100, 50);
        let expected = rgb_to_hsl(255, 100, 50);
        assert!(approx_eq_tuple3((h, s, l), expected, 0.01));
    }

    #[test]
    fn test_hsl_to_rgb() {
        // test pure red
        let (r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5);
        // result should be linear-space red
        assert!(approx_eq(r, 1.0, 0.01));
        assert!(approx_eq(g, 0.0, 0.01));
        assert!(approx_eq(b, 0.0, 0.01));

        // test gray (no saturation)
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 0.5);
        let expected_linear = 0.214; // 0.5 in sRGB converted to linear
        assert!(approx_eq(r, expected_linear, 0.01));
        assert!(approx_eq(g, expected_linear, 0.01));
        assert!(approx_eq(b, expected_linear, 0.01));

        // test clamping
        let result1 = hsl_to_rgb(-0.1, 2.0, 0.5);
        let result2 = hsl_to_rgb(0.0, 1.0, 0.5);
        assert!(approx_eq_tuple3(result1, result2, 0.01));
    }

    #[test]
    fn test_rgb_hsl_roundtrip() {
        // roundtrip through RGB -> HSL -> RGB (note: we need to account for color space)
        let test_colors = vec![(128, 64, 192), (255, 128, 0), (0, 255, 128)];

        for (r, g, b) in test_colors {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            // hsl_to_rgb returns linear space, so we need to convert back
            let (r_linear, g_linear, b_linear) = hsl_to_rgb(h, s, l);

            // convert linear back to sRGB for comparison
            let linear_to_srgb = |c: f32| -> u32 {
                let gamma = if c <= 0.0031308 {
                    c * 12.92
                } else {
                    1.055 * c.powf(1.0 / 2.4) - 0.055
                };
                (gamma * 255.0).round() as u32
            };

            let r_result = linear_to_srgb(r_linear);
            let g_result = linear_to_srgb(g_linear);
            let b_result = linear_to_srgb(b_linear);

            // allow some tolerance due to floating point conversions
            assert!((r_result as i32 - r as i32).abs() <= 2);
            assert!((g_result as i32 - g as i32).abs() <= 2);
            assert!((b_result as i32 - b as i32).abs() <= 2);
        }
    }

    #[test]
    fn test_rgb_to_p3() {
        // test that sRGB white maps to P3 white
        let (r, g, b) = rgb_to_p3(1.0, 1.0, 1.0);
        assert!(approx_eq(r, 1.0, 0.01));
        assert!(approx_eq(g, 1.0, 0.01));
        assert!(approx_eq(b, 1.0, 0.01));

        // test that sRGB black maps to P3 black
        let (r, g, b) = rgb_to_p3(0.0, 0.0, 0.0);
        assert!(approx_eq(r, 0.0, 0.01));
        assert!(approx_eq(g, 0.0, 0.01));
        assert!(approx_eq(b, 0.0, 0.01));

        // test pure red (should be slightly different in P3)
        let (r, g, b) = rgb_to_p3(1.0, 0.0, 0.0);
        assert!(r > 0.7 && r <= 1.0);
        assert!(g >= 0.0 && g < 0.25); // adjusted for actual matrix values
        assert!(b >= 0.0 && b < 0.15); // adjusted for actual transform result
    }

    #[test]
    fn test_p3_to_rgb() {
        // test that P3 white maps to sRGB white
        let (r, g, b) = p3_to_rgb(1.0, 1.0, 1.0);
        assert!(approx_eq(r, 1.0, 0.01));
        assert!(approx_eq(g, 1.0, 0.01));
        assert!(approx_eq(b, 1.0, 0.01));

        // test that P3 black maps to sRGB black
        let (r, g, b) = p3_to_rgb(0.0, 0.0, 0.0);
        assert!(approx_eq(r, 0.0, 0.01));
        assert!(approx_eq(g, 0.0, 0.01));
        assert!(approx_eq(b, 0.0, 0.01));
    }

    #[test]
    fn test_rgb_p3_roundtrip() {
        // test roundtrip conversion
        let test_colors = vec![(0.5, 0.3, 0.7), (0.8, 0.2, 0.4), (0.1, 0.9, 0.5)];

        for (r, g, b) in test_colors {
            let (rp3, gp3, bp3) = rgb_to_p3(r, g, b);
            let (r2, g2, b2) = p3_to_rgb(rp3, gp3, bp3);

            // should be very close after roundtrip
            assert!(approx_eq(r, r2, 0.02));
            assert!(approx_eq(g, g2, 0.02));
            assert!(approx_eq(b, b2, 0.02));
        }
    }

    #[test]
    fn test_hsl_to_p3() {
        // test pure hues
        let (r, _g, _b) = hsl_to_p3(0.0, 1.0, 0.5); // red
        assert!(r > 0.8);

        let (_r, g, _b) = hsl_to_p3(0.333, 1.0, 0.5); // green
        assert!(g > 0.8);

        let (_r, _g, b) = hsl_to_p3(0.667, 1.0, 0.5); // blue
        assert!(b > 0.8);

        // test gray (should be same in both color spaces)
        let (r, g, b) = hsl_to_p3(0.0, 0.0, 0.5);
        assert!(approx_eq(r, 0.5, 0.01));
        assert!(approx_eq(g, 0.5, 0.01));
        assert!(approx_eq(b, 0.5, 0.01));
    }

    #[test]
    fn test_p3_to_hsl() {
        // test roundtrip through P3 -> HSL -> P3
        let test_colors = vec![(0.8, 0.2, 0.4), (0.3, 0.7, 0.5), (0.5, 0.5, 0.5)];

        for (rp3, gp3, bp3) in test_colors {
            let (h, s, l) = p3_to_hsl(rp3, gp3, bp3);
            let (r2, g2, b2) = hsl_to_p3(h, s, l);

            // should be close after roundtrip
            assert!(approx_eq(rp3, r2, 0.03));
            assert!(approx_eq(gp3, g2, 0.03));
            assert!(approx_eq(bp3, b2, 0.03));
        }
    }
}
