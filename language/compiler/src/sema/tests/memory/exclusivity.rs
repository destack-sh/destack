use crate::tests::{DirRows, TestSession};

/// Reject a Copy bound for a struct with an inline Drop conformance.
#[test]
fn test_reject_copy_bound_for_inline_drop_struct() {
    let session = TestSession::single(
        r#"
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
    drop(&exclusive this): void {}
}

declare function duplicate<T: Copy>(value: T): void;
declare const guard: Guard;

duplicate(guard);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
    drop(&exclusive this): void {}
}

declare function duplicate<T: Copy>(value: T): void;
declare const guard: Guard;

duplicate(guard);

=== dir ===
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
/// @type.symbol symbol=Guard type=Guard
/// @definition.struct symbol=Guard
/// @definition.where symbol=Guard source=Drop relation=satisfies left=this right=memory.drop.Drop
/// @definition.implements symbol=Guard source=Drop target=memory.drop.Drop
/// @definition.method symbol=Guard.drop source="drop(&exclusive this): void {}" slot=drop type=<Guard.drop.'a>(this: &Guard.drop.'a exclusive this) => void
/// @definition.conformance symbol=Guard member=Guard.drop requirement=memory.drop.Drop.drop
/// @resolution.name source=Drop target=memory.drop.Drop

    drop(&exclusive this): void {}
    /// @generic.template symbol=Guard.drop parent=template#0 parameters=('a)
    /// @type.symbol symbol=Guard.drop source="drop(&exclusive this): void {}" type=<Guard.drop.'a>(this: &Guard.drop.'a exclusive this) => void
    /// @type.symbol symbol=Guard.drop.this source="&exclusive this" type=&Guard.drop.'a exclusive this

}

declare function duplicate<T: Copy>(value: T): void;
/// @generic.template symbol=duplicate parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=duplicate source="declare function duplicate<T: Copy>(value: T): void" type=<T: memory.capability.Copy>(T) => void
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T

declare const guard: Guard;
/// @type.symbol symbol=guard source=guard type=Guard
/// @resolution.pattern source=guard kind=binding target=guard
/// @resolution.name source=Guard target=Guard

duplicate(guard);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(guard) parameters=(<error>) arguments=(provided(guard) as <error>) return=void kind=symbol target=duplicate instance=duplicate<<error>>
/// @generic.instantiation id=duplicate<<error>> template=duplicate arguments=(<error>)
/// @resolution.name source=guard target=guard
/// @resolution.place source=guard placement="local" lifetime="static" access="readonly"
/// @resolution.access source=guard root=guard
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Guard' does not satisfy 'Copy'"
/// @diagnostic.label line=11 column=1 span="duplicate(guard)" line_source="duplicate(guard);"
/// @diagnostic.related line=8 column=28 span="T" line_source="declare function duplicate<T: Copy>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

/// Reject a Copy bound for a struct with an extension-declared Drop conformance.
#[test]
fn test_reject_copy_bound_for_extension_drop_struct() {
    let session = TestSession::single(
        r#"
import { Copy, Drop } from "destack:memory";

struct Guard {
    handle: int32;
}

extension of Guard implements Drop {
    drop(&exclusive this): void {}
}

declare function duplicate<T: Copy>(value: T): void;
declare const guard: Guard;

duplicate(guard);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy, Drop } from "destack:memory";

struct Guard {
    handle: int32;
}

extension of Guard implements Drop {
    drop(&exclusive this): void {}
}

declare function duplicate<T: Copy>(value: T): void;
declare const guard: Guard;

duplicate(guard);

=== dir ===
import { Copy, Drop } from "destack:memory";

struct Guard {
/// @type.symbol symbol=Guard type=Guard
/// @definition.struct symbol=Guard
/// @definition.field symbol=Guard.handle source="handle: int32" key=handle type=int32

    handle: int32;
    /// @type.symbol symbol=Guard.handle source="handle: int32" type=int32

}

extension of Guard implements Drop {
/// @definition.extension symbol=<module>#2 form=local target=Guard
/// @definition.implements symbol=<module>#2 source=Drop target=memory.drop.Drop
/// @definition.method symbol=drop source="drop(&exclusive this): void {}" slot=drop type=<drop.'a>(this: &drop.'a exclusive this) => void
/// @definition.conformance symbol=<module>#2 member=drop requirement=memory.drop.Drop.drop
/// @resolution.name source=Guard target=Guard
/// @resolution.name source=Drop target=memory.drop.Drop

    drop(&exclusive this): void {}
    /// @generic.template symbol=drop parent=template#0 parameters=('a)
    /// @type.symbol symbol=drop source="drop(&exclusive this): void {}" type=<drop.'a>(this: &drop.'a exclusive this) => void
    /// @type.symbol symbol=drop.this source="&exclusive this" type=&drop.'a exclusive this

}

declare function duplicate<T: Copy>(value: T): void;
/// @generic.template symbol=duplicate parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=duplicate source="declare function duplicate<T: Copy>(value: T): void" type=<T: memory.capability.Copy>(T) => void
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T

declare const guard: Guard;
/// @type.symbol symbol=guard source=guard type=Guard
/// @resolution.pattern source=guard kind=binding target=guard
/// @resolution.name source=Guard target=Guard

duplicate(guard);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(guard) parameters=(<error>) arguments=(provided(guard) as <error>) return=void kind=symbol target=duplicate instance=duplicate<<error>>
/// @generic.instantiation id=duplicate<<error>> template=duplicate arguments=(<error>)
/// @resolution.name source=guard target=guard
/// @resolution.place source=guard placement="local" lifetime="static" access="readonly"
/// @resolution.access source=guard root=guard
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'Guard' does not satisfy 'Copy'"
/// @diagnostic.label line=15 column=1 span="duplicate(guard)" line_source="duplicate(guard);"
/// @diagnostic.related line=12 column=28 span="T" line_source="declare function duplicate<T: Copy>(value: T): void;" message="required by this bound on 'T'"
"#,
    );
}

/// Satisfy a Copy bound for a plain struct beside its Drop-conforming sibling.
#[test]
fn test_satisfy_copy_bound_for_plain_struct() {
    let session = TestSession::single(
        r#"
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
    drop(&exclusive this): void {}
}

struct Plain {
    value: int32;
}

declare function duplicate<T: Copy>(value: T): void;
declare const plain: Plain;

duplicate(plain);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
    drop(&exclusive this): void {}
}

struct Plain {
    value: int32;
}

declare function duplicate<T: Copy>(value: T): void;
declare const plain: Plain;

duplicate<Plain>(plain);

=== dir ===
import { Copy, Drop } from "destack:memory";

struct Guard implements Drop {
/// @type.symbol symbol=Guard type=Guard
/// @definition.struct symbol=Guard
/// @definition.where symbol=Guard source=Drop relation=satisfies left=this right=memory.drop.Drop
/// @definition.implements symbol=Guard source=Drop target=memory.drop.Drop
/// @definition.method symbol=Guard.drop source="drop(&exclusive this): void {}" slot=drop type=<Guard.drop.'a>(this: &Guard.drop.'a exclusive this) => void
/// @definition.conformance symbol=Guard member=Guard.drop requirement=memory.drop.Drop.drop
/// @resolution.name source=Drop target=memory.drop.Drop

    drop(&exclusive this): void {}
    /// @generic.template symbol=Guard.drop parent=template#0 parameters=('a)
    /// @type.symbol symbol=Guard.drop source="drop(&exclusive this): void {}" type=<Guard.drop.'a>(this: &Guard.drop.'a exclusive this) => void
    /// @type.symbol symbol=Guard.drop.this source="&exclusive this" type=&Guard.drop.'a exclusive this

}

struct Plain {
/// @type.symbol symbol=Plain type=Plain
/// @definition.struct symbol=Plain
/// @definition.field symbol=Plain.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Plain.value source="value: int32" type=int32

}

declare function duplicate<T: Copy>(value: T): void;
/// @generic.template symbol=duplicate parameters=(T: memory.capability.Copy)
/// @type.symbol symbol=duplicate source="declare function duplicate<T: Copy>(value: T): void" type=<T: memory.capability.Copy>(T) => void
/// @type.symbol symbol=duplicate.T source="T: Copy" type=T
/// @resolution.name source=Copy target=memory.capability.Copy
/// @type.symbol symbol=duplicate.value source="value: T" type=T
/// @resolution.name source=T target=duplicate.T

declare const plain: Plain;
/// @type.symbol symbol=plain source=plain type=Plain
/// @resolution.pattern source=plain kind=binding target=plain
/// @resolution.name source=Plain target=Plain

duplicate(plain);
/// @resolution.name source=duplicate target=duplicate
/// @resolution.call source=duplicate(plain) parameters=(Plain) arguments=(provided(plain) as Plain) return=void kind=symbol target=duplicate instance=duplicate<Plain>
/// @generic.instantiation id=duplicate<Plain> template=duplicate arguments=(Plain)
/// @resolution.name source=plain target=plain
/// @resolution.place source=plain placement="local" lifetime="static" access="readonly"
/// @resolution.access source=plain root=plain
"#,
        r#"
"#,
    );
}
