#![no_main]

use destack_time::Duration;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(d) = s.parse::<Duration>()
    {
        let formatted = d.to_string();
        if let Ok(parsed_again) = formatted.parse::<Duration>() {
            assert_eq!(parsed_again, d);
        } else {
            panic!("formatted duration did not parse back: {formatted}");
        }
    }
});


