use crate::tests::{DirRows, TestSession};

/// Read a class member kept by a true `@if` guard.
#[test]
fn test_static_true_class_member_is_available() {
    let session = TestSession::single(
        r#"
struct NarrowMeta {}
struct WideMeta {}

class Segment {
    @if(true)
    narrow: NarrowMeta = NarrowMeta {};

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
}

declare const segment: Segment;
const narrowMeta = segment.narrow;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types().with_statics(),
        r#"
=== annotated ===
struct NarrowMeta {}
struct WideMeta {}

class Segment {
    @if(true)
    narrow: NarrowMeta = NarrowMeta {};

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
}

declare const segment: Segment;
const narrowMeta: NarrowMeta = segment.narrow;

=== dir ===
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta source="struct NarrowMeta {}" type=NarrowMeta
/// @definition.struct symbol=NarrowMeta source="struct NarrowMeta {}"

struct WideMeta {}
/// @type.symbol symbol=WideMeta source="struct WideMeta {}" type=WideMeta
/// @definition.struct symbol=WideMeta source="struct WideMeta {}"

class Segment {
/// @type.symbol symbol=Segment type=typeof Segment
/// @definition.class symbol=Segment
/// @definition.field symbol=Segment.narrow source="narrow: NarrowMeta = NarrowMeta {}" key=narrow type=NarrowMeta
/// @definition.field symbol=Segment.value source="value: int32 = 0" key=value type=int32

    @if(true)
    narrow: NarrowMeta = NarrowMeta {};
    /// @type.symbol symbol=Segment.narrow source="narrow: NarrowMeta = NarrowMeta {}" type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta
    /// @type.node source="NarrowMeta {}" type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
    /// @type.symbol symbol=Segment.value source="value: int32 = 0" type=int32
    /// @type.node source=0 type=0

}

declare const segment: Segment;
/// @type.symbol symbol=segment source=segment type=Segment
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=Segment target=Segment

const narrowMeta = segment.narrow;
/// @type.symbol symbol=narrowMeta source=narrowMeta type=NarrowMeta
/// @resolution.pattern source=narrowMeta kind=binding target=narrowMeta
/// @type.node source=segment type=Segment
/// @type.node source=segment.narrow type=NarrowMeta
/// @resolution.name source=segment target=segment
/// @resolution.member source=segment.narrow receiver=Segment type=NarrowMeta kind=field target_receiver=Segment key=narrow target=Segment.narrow target_type=NarrowMeta
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
/// @resolution.access source=segment.narrow root=segment keys=[narrow]
"#,
    );
}

/// Reject a read of a class member dropped by a false `@if` guard.
#[test]
fn test_static_false_class_member_is_unavailable() {
    let session = TestSession::single(
        r#"
struct NarrowMeta {}
struct WideMeta {}

class Segment {
    @if(true)
    narrow: NarrowMeta = NarrowMeta {};

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
}

declare const segment: Segment;
segment.wide;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_statics(),
        r#"
=== annotated ===
struct NarrowMeta {}
struct WideMeta {}

class Segment {
    @if(true)
    narrow: NarrowMeta = NarrowMeta {};

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
}

declare const segment: Segment;
segment.wide;

=== dir ===
struct NarrowMeta {}
/// @type.symbol symbol=NarrowMeta source="struct NarrowMeta {}" type=NarrowMeta
/// @definition.struct symbol=NarrowMeta source="struct NarrowMeta {}"

struct WideMeta {}
/// @type.symbol symbol=WideMeta source="struct WideMeta {}" type=WideMeta
/// @definition.struct symbol=WideMeta source="struct WideMeta {}"

class Segment {
/// @type.symbol symbol=Segment type=typeof Segment
/// @definition.class symbol=Segment
/// @definition.field symbol=Segment.narrow source="narrow: NarrowMeta = NarrowMeta {}" key=narrow type=NarrowMeta
/// @definition.field symbol=Segment.value source="value: int32 = 0" key=value type=int32

    @if(true)
    narrow: NarrowMeta = NarrowMeta {};
    /// @type.symbol symbol=Segment.narrow source="narrow: NarrowMeta = NarrowMeta {}" type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta
    /// @type.node source="NarrowMeta {}" type=NarrowMeta
    /// @resolution.name source=NarrowMeta target=NarrowMeta

    @if(false)
    wide: WideMeta;

    value: int32 = 0;
    /// @type.symbol symbol=Segment.value source="value: int32 = 0" type=int32
    /// @type.node source=0 type=0

}

declare const segment: Segment;
/// @type.symbol symbol=segment source=segment type=Segment
/// @resolution.pattern source=segment kind=binding target=segment
/// @resolution.name source=Segment target=Segment

segment.wide;
/// @type.node source=segment type=Segment
/// @type.node source=segment.wide type=<error>
/// @resolution.name source=segment target=segment
/// @resolution.place source=segment placement="local" lifetime="static" access="immutable"
/// @resolution.access source=segment root=segment
/// @resolution.rejected source=segment.wide
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'wide' does not exist on type 'Segment'"
/// @diagnostic.label line=16 column=9 span="wide" line_source="segment.wide;"
"#,
    );
}

/// Reject an `@if` guard that depends on a class type parameter.
#[test]
fn test_static_if_rejects_generic_dependent_class_member_guard() {
    let session = TestSession::single(
        r#"
struct TextMeta {}

class Packet<T> {
    @if(T extends string)
    meta: TextMeta;

    value: T;

    constructor(value: T) {
        this.value = value;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types().with_statics(),
        r#"
=== annotated ===
struct TextMeta {}

class Packet<in out T> {
    @if(T extends string)
    meta: TextMeta;

    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

=== dir ===
struct TextMeta {}
/// @type.symbol symbol=TextMeta source="struct TextMeta {}" type=TextMeta
/// @definition.struct symbol=TextMeta source="struct TextMeta {}"

class Packet<T> {
/// @generic.template symbol=Packet parameters=(in out T)
/// @type.symbol symbol=Packet type=typeof Packet
/// @definition.class symbol=Packet template=(in out T)
/// @definition.field symbol=Packet.value source="value: T" key=value type=T
/// @definition.method symbol=Packet.constructor slot=constructor role=constructor type=(this: &'managed Packet<T>, T) => Packet<T>
/// @type.symbol symbol=Packet.T source=T type=T

    @if(T extends string)
    meta: TextMeta;

    value: T;
    /// @type.symbol symbol=Packet.value source="value: T" type=T
    /// @resolution.name source=T target=Packet.T

    constructor(value: T) {
    /// @type.symbol symbol=Packet.constructor type=(this: &'managed Packet<T>, T) => Packet<T>
    /// @type.symbol symbol=Packet.constructor.this type=&'managed Packet<T>
    /// @type.symbol symbol=Packet.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Packet.T

        this.value = value;
        /// @type.node source="this.value = value" type=T
        /// @type.node source=this type=&'managed Packet<T>
        /// @type.node source=this.value type=T
        /// @resolution.receiver source=this kind=this declaration=Packet type=&'managed Packet<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Packet<T>, target=field(receiver=&'managed Packet<T>, target=Packet.value, type=T), type=T" type=T
        /// @type.node source=value type=T
        /// @resolution.name source=value target=Packet.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Packet.constructor.value

    }
}
"#,
        r#"
/// @diagnostic.error id=undecidable-static-condition message="static @if condition must be statically decidable"
/// @diagnostic.label line=5 column=11 span="extends" line_source="@if(T extends string)"
"#,
    );
}
