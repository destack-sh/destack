use crate::tests::TestSession;

#[test]
fn test_lower_scalar_union_entries_to_tagged_variants() {
    let session = TestSession::single(
        r#"
function pick(flag: boolean, count: int32): int32 | boolean {
    if (flag) {
        return count;
    }
    return 5;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.pick(v0: boolean, v1: int32): variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } {
entry(v0: boolean, v1: int32):
    branch v0, b1, b2

b1:
    v2: variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v1
    return v2

b2:
    v3: int32 = 5
    v4: variant<uint8, int32> { 0uint8 = int32; 1uint8 = boolean; } = variant.new 0, v3
    return v4
}
/// @layout.variant name=type@5 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@16 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@21 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_tagged_newtype_to_a_variant_carrier() {
    let session = TestSession::single(
        r#"
struct Circle {
    kind: "circle";
    radius: float64;
}

struct Square {
    kind: "square";
    side: int32;
}

@derive(Tagged)
newtype Shape = Circle | Square;

function keep(shape: Shape): Shape {
    return shape;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Circle {
    kind: void;
    radius: float64;
}

@copy
type Square {
    kind: void;
    side: int32;
}

@copy
type Shape = variant<uint8, Circle> { 0uint8 = Circle; 1uint8 = Square; };

function main.keep(v0: Shape): Shape {
entry(v0: Shape):
    return v0
}
/// @layout.struct name=Circle size=8 align=8 fields=(kind@8+0, radius@0+8)
/// @layout.struct name=Square size=4 align=4 fields=(kind@4+0, side@0+4)
/// @layout.variant name=Shape size=16 align=8 encoding=direct(tag@0+1) cases=(0@8, 1@8)
"#,
    );
}

#[test]
fn test_lower_variant_identity_compares_shared_case_payloads() {
    let session = TestSession::single(
        r#"
newtype Count = int32;
newtype Flag = boolean;

function same(left: Count | Flag, right: Count | Flag): boolean {
    return left === right;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Count = newtype<int32>;

@copy
type Flag = newtype<boolean>;

function main.same(v0: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }, v1: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }): boolean {
    local l0: boolean, readonly

entry(v0: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }, v1: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }):
    v2: uint8 = variant.tag v0
    v3: uint8 = variant.tag v1
    v4: boolean = int.eq v2, v3
    branch v4, b1, b2

b1:
    variant.switch v0, 0 => b4, 1 => b5, else b2

b2:
    v15: boolean = false
    local.set l0, v15
    jump b3

b3:
    v16: boolean = local.get l0
    return v16

b4:
    v5: Count = variant.payload v0, 0
    v6: int32 = field.get v5, 0
    v7: Count = variant.payload v1, 0
    v8: int32 = field.get v7, 0
    v9: boolean = int.eq v6, v8
    local.set l0, v9
    jump b3

b5:
    v10: Flag = variant.payload v0, 1
    v11: boolean = field.get v10, 0
    v12: Flag = variant.payload v1, 1
    v13: boolean = field.get v12, 0
    v14: boolean = int.eq v11, v13
    local.set l0, v14
    jump b3
}
/// @layout.variant name=type@7 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@9 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_reordered_union_identity_maps_case_indices() {
    let session = TestSession::single(
        r#"
newtype Count = int32;
newtype Flag = boolean;

function same(left: Count | Flag, right: Flag | Count): boolean {
    return left === right;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Count = newtype<int32>;

@copy
type Flag = newtype<boolean>;

function main.same(v0: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }, v1: variant<uint8, Flag> { 0uint8 = Flag; 1uint8 = Count; }): boolean {
    local l0: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }, readonly
    local l1: boolean, readonly

entry(v0: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }, v1: variant<uint8, Flag> { 0uint8 = Flag; 1uint8 = Count; }):
    variant.switch v1, 0 => b2, 1 => b3

b1:
    v6: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = local.get l0
    v7: uint8 = variant.tag v0
    v8: uint8 = variant.tag v6
    v9: boolean = int.eq v7, v8
    branch v9, b4, b5

b2:
    v2: Flag = variant.payload v1, 0
    v3: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = variant.new 1, v2
    local.set l0, v3
    jump b1

b3:
    v4: Count = variant.payload v1, 1
    v5: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = variant.new 0, v4
    local.set l0, v5
    jump b1

b4:
    variant.switch v0, 0 => b7, 1 => b8, else b5

b5:
    v20: boolean = false
    local.set l1, v20
    jump b6

b6:
    v21: boolean = local.get l1
    return v21

b7:
    v10: Count = variant.payload v0, 0
    v11: int32 = field.get v10, 0
    v12: Count = variant.payload v6, 0
    v13: int32 = field.get v12, 0
    v14: boolean = int.eq v11, v13
    local.set l1, v14
    jump b6

b8:
    v15: Flag = variant.payload v0, 1
    v16: boolean = field.get v15, 0
    v17: Flag = variant.payload v6, 1
    v18: boolean = field.get v17, 0
    v19: boolean = int.eq v16, v18
    local.set l1, v19
    jump b6
}
/// @layout.variant name=type@7 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@9 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@15 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_union_conversion_applies_case_adjustments() {
    let session = TestSession::single(
        r#"
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds", r#"
@copy
type Flag = newtype<boolean>;

function main.widen(v0: variant<uint8, void> { 0uint8 = void; 1uint8 = Flag; }): variant<uint8, int32> { 0uint8 = int32; 1uint8 = Flag; } {
    local l0: variant<uint8, int32> { 0uint8 = int32; 1uint8 = Flag; }, readonly

entry(v0: variant<uint8, void> { 0uint8 = void; 1uint8 = Flag; }):
    variant.switch v0, 0 => b2, 1 => b3

b1:
    v5: variant<uint8, int32> { 0uint8 = int32; 1uint8 = Flag; } = local.get l0
    return v5

b2:
    v1: int32 = 1
    v2: variant<uint8, int32> { 0uint8 = int32; 1uint8 = Flag; } = variant.new 0, v1
    local.set l0, v2
    jump b1

b3:
    v3: Flag = variant.payload v0, 1
    v4: variant<uint8, int32> { 0uint8 = int32; 1uint8 = Flag; } = variant.new 1, v3
    local.set l0, v4
    jump b1
}
/// @layout.variant name=type@5 size=2 align=1 encoding=direct(tag@0+1) cases=(0@1, 1@1)
/// @layout.variant name=type@8 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@14 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_imported_union_identity_uses_checked_case_correspondence() {
    let session = TestSession::builder()
        .module(
            "value.ds",
            r#"
export newtype Count = int32;
export newtype Flag = boolean;
export type Value = Count | Flag;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Count, Flag, Value } from "./value";

function same(left: Value, right: Count | Flag): boolean {
    return left === right;
}
"#,
        )
        .build();

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type value.Count = newtype<int32>;

@copy
type value.Flag = newtype<boolean>;

function main.same(v0: variant<uint8, value.Count> { 0uint8 = value.Count; 1uint8 = value.Flag; }, v1: variant<uint8, value.Count> { 0uint8 = value.Count; 1uint8 = value.Flag; }): boolean {
    local l0: boolean, readonly

entry(v0: variant<uint8, value.Count> { 0uint8 = value.Count; 1uint8 = value.Flag; }, v1: variant<uint8, value.Count> { 0uint8 = value.Count; 1uint8 = value.Flag; }):
    v2: uint8 = variant.tag v0
    v3: uint8 = variant.tag v1
    v4: boolean = int.eq v2, v3
    branch v4, b1, b2

b1:
    variant.switch v0, 0 => b4, 1 => b5, else b2

b2:
    v15: boolean = false
    local.set l0, v15
    jump b3

b3:
    v16: boolean = local.get l0
    return v16

b4:
    v5: value.Count = variant.payload v0, 0
    v6: int32 = field.get v5, 0
    v7: value.Count = variant.payload v1, 0
    v8: int32 = field.get v7, 0
    v9: boolean = int.eq v6, v8
    local.set l0, v9
    jump b3

b5:
    v10: value.Flag = variant.payload v0, 1
    v11: boolean = field.get v10, 0
    v12: value.Flag = variant.payload v1, 1
    v13: boolean = field.get v12, 0
    v14: boolean = int.eq v11, v13
    local.set l0, v14
    jump b3
}
/// @layout.variant name=type@7 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
/// @layout.variant name=type@9 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_transparent_newtype_identity_uses_backing_cases() {
    let session = TestSession::single(
        r#"
newtype Count = int32;
newtype Flag = boolean;
newtype Value = Count | Flag;

function same(left: Value, right: Value): boolean {
    return left === right;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Count = newtype<int32>;

@copy
type Flag = newtype<boolean>;

@copy
type Value = newtype<variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }>;

function main.same(v0: Value, v1: Value): boolean {
    local l0: boolean, readonly

entry(v0: Value, v1: Value):
    v2: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = field.get v0, 0
    v3: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = field.get v1, 0
    v4: uint8 = variant.tag v2
    v5: uint8 = variant.tag v3
    v6: boolean = int.eq v4, v5
    branch v6, b1, b2

b1:
    variant.switch v2, 0 => b4, 1 => b5, else b2

b2:
    v17: boolean = false
    local.set l0, v17
    jump b3

b3:
    v18: boolean = local.get l0
    return v18

b4:
    v7: Count = variant.payload v2, 0
    v8: int32 = field.get v7, 0
    v9: Count = variant.payload v3, 0
    v10: int32 = field.get v9, 0
    v11: boolean = int.eq v8, v10
    local.set l0, v11
    jump b3

b5:
    v12: Flag = variant.payload v2, 1
    v13: boolean = field.get v12, 0
    v14: Flag = variant.payload v3, 1
    v15: boolean = field.get v14, 0
    v16: boolean = int.eq v13, v15
    local.set l0, v16
    jump b3
}
/// @layout.variant name=type@7 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_nested_transparent_newtype_identity_uses_backing_cases() {
    let session = TestSession::single(
        r#"
newtype Count = int32;
newtype Flag = boolean;
newtype Inner = Count | Flag;
newtype Outer = Inner;

function same(left: Outer, right: Outer): boolean {
    return left === right;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
@copy
type Count = newtype<int32>;

@copy
type Flag = newtype<boolean>;

@copy
type Inner = newtype<variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; }>;

@copy
type Outer = newtype<Inner>;

function main.same(v0: Outer, v1: Outer): boolean {
    local l0: boolean, readonly

entry(v0: Outer, v1: Outer):
    v2: Inner = field.get v0, 0
    v3: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = field.get v2, 0
    v4: Inner = field.get v1, 0
    v5: variant<uint8, Count> { 0uint8 = Count; 1uint8 = Flag; } = field.get v4, 0
    v6: uint8 = variant.tag v3
    v7: uint8 = variant.tag v5
    v8: boolean = int.eq v6, v7
    branch v8, b1, b2

b1:
    variant.switch v3, 0 => b4, 1 => b5, else b2

b2:
    v19: boolean = false
    local.set l0, v19
    jump b3

b3:
    v20: boolean = local.get l0
    return v20

b4:
    v9: Count = variant.payload v3, 0
    v10: int32 = field.get v9, 0
    v11: Count = variant.payload v5, 0
    v12: int32 = field.get v11, 0
    v13: boolean = int.eq v10, v12
    local.set l0, v13
    jump b3

b5:
    v14: Flag = variant.payload v3, 1
    v15: boolean = field.get v14, 0
    v16: Flag = variant.payload v5, 1
    v17: boolean = field.get v16, 0
    v18: boolean = int.eq v15, v17
    local.set l0, v18
    jump b3
}
/// @layout.variant name=type@7 size=8 align=4 encoding=direct(tag@0+1) cases=(0@4, 1@4)
"#,
    );
}

#[test]
fn test_lower_boolean_undefined_union_into_a_niche() {
    let session = TestSession::single(
        r#"
function keep(value: boolean | undefined): boolean | undefined {
    return value;
}

function forget(): boolean | undefined {
    return undefined;
}
"#,
    );

    session.assert_mir_lowered(
        "main.ds",
        r#"
function main.keep(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }): variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } {
entry(v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; }):
    return v0
}

function main.forget(): variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } {
entry:
    v0: variant<uint8, boolean> { 0uint8 = boolean; 1uint8 = void; } = variant.new 1
    return v0
}
/// @layout.variant name=type@3 size=1 align=1 encoding=niche(@0+1, start=2) cases=(0@0, 1@0)
/// @layout.variant name=type@7 size=1 align=1 encoding=niche(@0+1, start=2) cases=(0@0, 1@0)
/// @layout.variant name=type@12 size=1 align=1 encoding=niche(@0+1, start=2) cases=(0@0, 1@0)
/// @layout.variant name=type@21 size=1 align=1 encoding=niche(@0+1, start=2) cases=(0@0, 1@0)
"#,
    );
}
