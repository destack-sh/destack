#![no_main]

use destack_json::{FormatOptions, JsonValue, format_json, parse_json};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(v) = parse_json(s)
    {
        // compact roundtrip
        let compact = format_json(&v, &FormatOptions::compact());
        if let Ok(v2) = parse_json(&compact) {
            assert_eq!(v2, v);
        } else {
            panic!("formatted JSON (compact) did not parse back: {compact}");
        }

        // pretty formatting should also parse
        let pretty = format_json(&v, &FormatOptions::pretty(2));
        if let Ok(v3) = parse_json(&pretty) {
            assert_eq!(v3, v);
        } else {
            panic!("formatted JSON (pretty) did not parse back: {pretty}");
        }
    }
});
