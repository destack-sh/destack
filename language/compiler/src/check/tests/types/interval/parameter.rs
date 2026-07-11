use crate::tests::{DirRows, TestSession};

#[test]
fn test_interval_constrains_static_parameter() {
    let session = TestSession::single(
        r#"
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

const ok: InlineBuffer<uint8, 16> = InlineBuffer<uint8, 16> {
    storage: [0; 16],
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
struct InlineBuffer<out T, comptime N: 0..=4096> {
    storage: [T; N];
}

const ok: InlineBuffer<uint8, 16> = InlineBuffer<uint8, 16> {
    storage: [0; 16],
};

=== checked ===
struct InlineBuffer<T, comptime N: 0..=4096> {
/// @generic.template symbol=InlineBuffer parameters=(out T, comptime N: 0..=4096)
/// @type.symbol symbol=InlineBuffer type=InlineBuffer
/// @definition.struct symbol=InlineBuffer template=(out T, comptime N: 0..=4096)
/// @definition.field symbol=InlineBuffer.storage source="storage: [T; N]" key=storage type=FixedArray<T, N>
/// @type.symbol symbol=InlineBuffer.T source=T type=T
/// @type.symbol symbol=InlineBuffer.N source="comptime N: 0..=4096" type=N

    storage: [T; N];
    /// @type.symbol symbol=InlineBuffer.storage source="storage: [T; N]" type=FixedArray<T, N>
    /// @resolution.name source=T target=InlineBuffer.T
    /// @resolution.name source=N target=InlineBuffer.N

}

const ok: InlineBuffer<uint8, 16> = InlineBuffer<uint8, 16> {
/// @type.symbol symbol=ok source=ok type=InlineBuffer<uint8, 16>
/// @resolution.name source=InlineBuffer target=InlineBuffer
/// @resolution.name source=InlineBuffer target=InlineBuffer

    storage: [0; 16],
};

/// @generic.instance id="InlineBuffer<uint8, 16>" template=InlineBuffer arguments=(uint8, 16)
"#,
    );
}

#[test]
fn test_interval_static_parameter_rejects_out_of_range_argument() {
    let session = TestSession::single(
        r#"
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

type TooLarge = InlineBuffer<uint8, 4097>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
struct InlineBuffer<out T, comptime N: 0..=4096> {
    storage: [T; N];
}

type TooLarge = InlineBuffer<uint8, 4097>;

=== checked ===
struct InlineBuffer<T, comptime N: 0..=4096> {
/// @generic.template symbol=InlineBuffer parameters=(out T, comptime N: 0..=4096)
/// @type.symbol symbol=InlineBuffer type=InlineBuffer
/// @definition.struct symbol=InlineBuffer template=(out T, comptime N: 0..=4096)
/// @definition.field symbol=InlineBuffer.storage source="storage: [T; N]" key=storage type=FixedArray<T, N>
/// @type.symbol symbol=InlineBuffer.T source=T type=T
/// @type.symbol symbol=InlineBuffer.N source="comptime N: 0..=4096" type=N

    storage: [T; N];
    /// @type.symbol symbol=InlineBuffer.storage source="storage: [T; N]" type=FixedArray<T, N>
    /// @resolution.name source=T target=InlineBuffer.T
    /// @resolution.name source=N target=InlineBuffer.N

}

type TooLarge = InlineBuffer<uint8, 4097>;
/// @type.symbol symbol=TooLarge source="type TooLarge = InlineBuffer<uint8, 4097>" type=InlineBuffer<uint8, 4097>
/// @definition.type symbol=TooLarge source="type TooLarge = InlineBuffer<uint8, 4097>" value=InlineBuffer<uint8, 4097>
/// @resolution.name source=InlineBuffer target=InlineBuffer

/// @generic.instance id="InlineBuffer<uint8, 4097>" template=InlineBuffer arguments=(uint8, 4097)
"#,
        r#"
/// @diagnostic.error code=EC201 message="type '4097' does not satisfy '0..=4096'"
/// @diagnostic.label line=6 column=17 span="InlineBuffer" line_source="type TooLarge = InlineBuffer<uint8, 4097>;"
"#,
    );
}
