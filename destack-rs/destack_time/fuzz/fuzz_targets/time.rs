#![no_main]

use destack_time::Time;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(t) = s.parse::<Time>()
    {
        let formatted = t.to_string();
        if let Ok(parsed_again) = formatted.parse::<Time>() {
            assert_eq!(parsed_again, t);
        } else {
            panic!("formatted time did not parse back: {formatted}");
        }
    }
});
