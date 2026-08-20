mod format;
mod markdown;
mod roundtrip;
mod transform;

use libtest_mimic::Arguments;

/// Run the formatter fixtures through the standard Rust test interface.
fn main() {
    let arguments = Arguments::from_args();
    let fixture = markdown::fixture_directory();
    let mut trials = transform::trials(&fixture.join("transform")).unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    trials.extend(
        roundtrip::trials(&fixture.join("roundtrip")).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        }),
    );

    libtest_mimic::run(&arguments, trials).exit();
}
