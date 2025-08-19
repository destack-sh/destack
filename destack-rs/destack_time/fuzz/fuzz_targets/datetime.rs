#![no_main]

use destack_time::DateTime;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(dt) = s.parse::<DateTime>()
    {
        let formatted = dt.to_string();
        if let Ok(parsed_again) = formatted.parse::<DateTime>() {
            assert_eq!(parsed_again, dt);
        } else {
            panic!("formatted datetime did not parse back: {formatted}");
        }
    }
});
