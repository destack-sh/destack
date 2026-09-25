use crate::tests::{DirRows, TestSession};

/// A written call to a parking binding outside the parking protocol is rejected.
#[test]
fn test_reject_a_parking_call_outside_the_protocol() {
    let session = TestSession::single(
        r#"
@binding("test.park", { provider: "runtime", effect: "deterministic", requires: [], park: true })
declare function parkNow(): void;

function wait(): void {
    parkNow();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
@binding("test.park", { provider: "runtime", effect: "deterministic", requires: [], park: true })
declare function parkNow(): void;

function wait(): void {
    parkNow();
}

=== dir ===
@binding("test.park", { provider: "runtime", effect: "deterministic", requires: [], park: true })
/// @resolution.name source=binding target=binding

declare function parkNow(): void;
/// @type.symbol symbol=parkNow source="declare function parkNow(): void" type=() => void

function wait(): void {
/// @type.symbol symbol=wait type=() => void

    parkNow();
    /// @resolution.name source=parkNow target=parkNow
    /// @resolution.call source=parkNow() parameters=() return=void kind=symbol target=parkNow

}
"#,
        r#"
/// @diagnostic.error id=park-outside-protocol message="cannot park outside 'await' and 'yield'"
/// @diagnostic.label line=6 column=5 span="parkNow()" line_source="parkNow();"
"#,
    );
}
