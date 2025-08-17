#![no_main]

use destack_uuid::Uuid;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data)
        && let Ok(u) = s.parse::<Uuid>()
    {
        let formatted = u.to_string();
        if let Ok(parsed_again) = formatted.parse::<Uuid>() {
            assert_eq!(parsed_again, u);
        } else {
            panic!("formatted uuid did not parse back: {formatted}");
        }
    }
});


