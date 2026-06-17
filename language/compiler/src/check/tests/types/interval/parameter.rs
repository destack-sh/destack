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
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

const ok: InlineBuffer<uint8, 16> = InlineBuffer<uint8, 16> {
    storage: [0; 16],
};

=== checked ===
struct InlineBuffer<T, comptime N: 0..=4096> {
/// @generic.template symbol=InlineBuffer parameters=[T, comptime N: 0..=4096]
/// @type.symbol symbol=InlineBuffer type=InlineBuffer<T, N>
/// @definition.struct symbol=InlineBuffer template=LocalGenericTemplateId(0)
/// @definition.field symbol=InlineBuffer.storage source="storage: [T; N]" key=storage type=[T; N]
/// @type.symbol symbol=InlineBuffer.T source=T type=T
/// @type.symbol symbol=InlineBuffer.N source=N type=0..=4096

    storage: [T; N];
    /// @type.symbol symbol=InlineBuffer.storage source="storage: [T; N]" type=[T; N]
    /// @resolution.name source=T target=InlineBuffer.T
    /// @resolution.name source=N target=InlineBuffer.N

}

const ok: InlineBuffer<uint8, 16> = InlineBuffer<uint8, 16> {
/// @type.symbol symbol=ok source=ok type=InlineBuffer<uint8, 16>
/// @resolution.name source=InlineBuffer target=InlineBuffer
/// @resolution.name source=InlineBuffer target=InlineBuffer

    storage: [0; 16],
    /// @type.node source=[0; 16] type=[uint8; 16]
    /// @type.node source=0 type=uint8
    /// @type.node source=16 type=16

};
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
struct InlineBuffer<T, comptime N: 0..=4096> {
    storage: [T; N];
}

type TooLarge = InlineBuffer<uint8, 4097>;

=== checked ===
struct InlineBuffer<T, comptime N: 0..=4096> {
/// @generic.template symbol=InlineBuffer parameters=[T, comptime N: 0..=4096]
/// @type.symbol symbol=InlineBuffer type=InlineBuffer<T, N>
/// @definition.struct symbol=InlineBuffer template=LocalGenericTemplateId(0)
/// @definition.field symbol=InlineBuffer.storage source="storage: [T; N]" key=storage type=[T; N]
/// @type.symbol symbol=InlineBuffer.T source=T type=T
/// @type.symbol symbol=InlineBuffer.N source=N type=0..=4096

    storage: [T; N];
    /// @type.symbol symbol=InlineBuffer.storage source="storage: [T; N]" type=[T; N]
    /// @resolution.name source=T target=InlineBuffer.T
    /// @resolution.name source=N target=InlineBuffer.N

}

type TooLarge = InlineBuffer<uint8, 4097>;
/// @type.symbol symbol=TooLarge source="type TooLarge = InlineBuffer<uint8, 4097>" type=<error>
/// @definition.type symbol=TooLarge source="type TooLarge = InlineBuffer<uint8, 4097>" value=<error>
/// @resolution.name source=InlineBuffer target=InlineBuffer
"#,
        r#"
/// @diagnostic.error code=EC201 message="type '4097' does not satisfy '0..=4096'"
/// @diagnostic.label line=6 column=17 source="InlineBuffer<uint8, 4097>"
"#,
    );
}
