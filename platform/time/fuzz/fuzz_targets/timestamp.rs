#![no_main]

use destack_time::Timestamp;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(ts) = s.parse::<Timestamp>()
    {
        let formatted = ts.to_string();
        if let Ok(parsed_again) = formatted.parse::<Timestamp>() {
            assert_eq!(parsed_again, ts);
        } else {
            panic!("formatted timestamp did not parse back: {formatted}");
        }
    }
});
