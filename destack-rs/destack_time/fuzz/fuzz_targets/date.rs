#![no_main]

use destack_time::Date;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(d) = s.parse::<Date>()
    {
        let formatted = d.to_string();
        if let Ok(parsed_again) = formatted.parse::<Date>() {
            assert_eq!(parsed_again, d);
        } else {
            panic!("formatted date did not parse back: {formatted}");
        }
    }
});
