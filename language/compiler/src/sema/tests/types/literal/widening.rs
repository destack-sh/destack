use crate::tests::{DirRows, TestSession};

/// A let array widens its element literals.
#[test]
fn test_widen_element_literals_in_a_let_array() {
    let session = TestSession::single(
        r#"
let values = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int64[] = [1, 2];
const first: int64 = values[0];

=== dir ===
let values = [1, 2];
/// @type.symbol symbol=values source=values type=int64[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int64> template=Array arguments=(int64)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
/// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
/// @type.node source=[1, 2] type=int64[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
/// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=int64
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=int64[]
/// @type.node source=values[0] type=int64
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=int64 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=int64, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<int64, \"managed\" & \"local\">" template=index#2 arguments=(int64, "managed" & "local")
/// @generic.instance id="index#2<int64, \"bound0\" & \"local\">" template=index#2 arguments=(int64, "bound0" & "local")
/// @type.node source=0 type=0
"#,
    );
}

/// A contextual array keeps the union element type its annotation states.
#[test]
fn test_preserve_a_union_element_type_in_a_contextual_array() {
    let session = TestSession::single(
        r#"
const values: (1 | 2)[] = [1, 2];
const first = values[0];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const values: (1 | 2)[] = [1, 2];
const first: 1 | 2 = values[0];

=== dir ===
const values: (1 | 2)[] = [1, 2];
/// @type.symbol symbol=values source=values type=1 | 2[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id="Array<1 | 2>" template=Array arguments=(1 | 2)
/// @generic.instance id="sliceAssumeInit<MaybeUninit<1 | 2>>" template=sliceAssumeInit arguments=(MaybeUninit<1 | 2>)
/// @generic.instance id="sliceUninit<MaybeUninit<1 | 2>>" template=sliceUninit arguments=(MaybeUninit<1 | 2>)
/// @type.node source=[1, 2] type=1 | 2[]
/// @resolution.call source=[1, 2] parameters=(^Slice<1 | 2>) arguments=(rest(provided(1) as 1 | 2, provided(2) as 1 | 2) as 1 | 2) return=1 | 2[] kind=symbol target=arrayFromOwnedSlice instance="arrayFromOwnedSlice<1 | 2>"
/// @generic.instantiation id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)
/// @generic.instance id="arrayFromOwnedSlice<1 | 2>" template=arrayFromOwnedSlice arguments=(1 | 2)
/// @type.node source=1 type=1
/// @type.node source=2 type=2

const first = values[0];
/// @type.symbol symbol=first source=first type=1 | 2
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=values type=1 | 2[]
/// @type.node source=values[0] type=1 | 2
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="immutable"
/// @resolution.access source=values root=values
/// @resolution.subscript source=values[0] type=1 | 2 kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=1 | 2, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<1 | 2, \"managed\" & \"local\">" template=index#2 arguments=(1 | 2, "managed" & "local")
/// @generic.instance id="index#2<1 | 2, \"bound0\" & \"local\">" template=index#2 arguments=(1 | 2, "bound0" & "local")
/// @type.node source=0 type=0
"#,
    );
}

/// A literal contextually typed by a mixed union coerces to one arm.
#[test]
fn test_require_coercion_for_a_contextual_literal_in_a_mixed_union() {
    let session = TestSession::single(
        r#"
const value: number | boolean = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
const value: float64 | boolean = 1 as float64 | boolean;

=== dir ===
const value: number | boolean = 1;
/// @type.symbol symbol=value source=value type=float64 | boolean
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1
/// @coercion.node source=1 from=1 adjustments=[{ kind: union, target: float64 | boolean, cases: ({ source: 1, target: float64, adjustments: [{ kind: materialize, target: float64 }] }) }] origin=implicit
"#,
    );
}

/// A union coercion records the adjustment each arm takes.
#[test]
fn test_record_case_adjustments_for_a_union_coercion() {
    let session = TestSession::single(
        r#"
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
newtype Flag = boolean;

function widen(value: 1 | Flag): int32 | Flag {
    return value as int32 | Flag;
}

=== dir ===
newtype Flag = boolean;
/// @type.symbol symbol=Flag source="newtype Flag = boolean" type=Flag
/// @definition.newtype symbol=Flag source="newtype Flag = boolean" backing=boolean constructors=[(boolean) => Flag]

function widen(value: 1 | Flag): int32 | Flag {
/// @type.symbol symbol=widen type=(1 | Flag) => int32 | Flag
/// @type.symbol symbol=widen.value source="value: 1 | Flag" type=1 | Flag
/// @resolution.name source=Flag target=Flag
/// @resolution.name source=Flag target=Flag

    return value;
    /// @type.node source=value type=1 | Flag
    /// @resolution.name source=value target=widen.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=widen.value
    /// @coercion.node source=value from=1 | Flag adjustments=[{ kind: union, target: int32 | Flag, cases: ({ source: 1, target: int32, adjustments: [{ kind: materialize, target: int32 }] }, { source: Flag, target: Flag }) }] origin=implicit

}
"#,
    );
}

/// A union of literals coerces to one common target type.
#[test]
fn test_coerce_union_members_to_a_common_target() {
    let session = TestSession::single(
        r#"
function widen(value: 1 | 2): int32 {
    return value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types().with_coercion(),
        r#"
=== annotated ===
function widen(value: 1 | 2): int32 {
    return value as int32;
}

=== dir ===
function widen(value: 1 | 2): int32 {
/// @type.symbol symbol=widen type=(1 | 2) => int32
/// @type.symbol symbol=widen.value source="value: 1 | 2" type=1 | 2

    return value;
    /// @type.node source=value type=1 | 2
    /// @resolution.name source=value target=widen.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=widen.value
    /// @coercion.node source=value from=1 | 2 adjustments=[{ kind: union, target: int32, cases: ({ source: 1, target: int32, adjustments: [{ kind: materialize, target: int32 }] }, { source: 2, target: int32, adjustments: [{ kind: materialize, target: int32 }] }) }] origin=implicit

}
"#,
    );
}

/// A const binding keeps the literal union a conditional produces.
#[test]
fn test_preserve_a_literal_union_in_a_const_conditional() {
    let session = TestSession::single(
        r#"
const value = true ? 1 : 2;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 1 | 2 = true ? 1 : 2;

=== dir ===
const value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=1 | 2
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=1 | 2
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=15 span="true" line_source="const value = true ? 1 : 2;"
"#,
    );
}

/// A let binding widens the literal union a conditional produces.
#[test]
fn test_widen_a_literal_union_in_a_let_conditional() {
    let session = TestSession::single(
        r#"
let value = true ? 1 : 2;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int64 = true ? 1 : 2;

=== dir ===
let value = true ? 1 : 2;
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="true ? 1 : 2" type=int64
/// @type.node source=true type=true
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
        r#"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=2 column=13 span="true" line_source="let value = true ? 1 : 2;"
"#,
    );
}

/// Widen the literal result a closure returns without an annotation.
#[test]
fn test_widen_the_literal_result_of_an_unannotated_closure() {
    let session = TestSession::single(
        r#"
const label = () => "ready";
const exact = (): "ready" => "ready";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const label: () => string = (): string => "ready";
const exact: () => "ready" = (): "ready" => "ready";

=== dir ===
const label = () => "ready";
/// @type.symbol symbol=label source=label type=Function<(), string, "readonly">
/// @resolution.pattern source=label kind=binding target=label
/// @type.symbol symbol=symbol1 source="() => \"ready\"" type=Function<(), string, "readonly">
/// @type.node source="() => \"ready\"" type=Function<(), string, "readonly">
/// @type.node source="\"ready\"" type="ready"

const exact = (): "ready" => "ready";
/// @type.symbol symbol=exact source=exact type=Function<(), "ready", "readonly">
/// @resolution.pattern source=exact kind=binding target=exact
/// @type.symbol symbol=symbol3 source="(): \"ready\" => \"ready\"" type=Function<(), "ready", "readonly">
/// @type.node source="(): \"ready\" => \"ready\"" type=Function<(), "ready", "readonly">
/// @type.node source="\"ready\"" type="ready"
"#,
        r#"

"#,
    );
}

/// Drop excess property checking once an object literal is read back through a binding.
#[test]
fn test_drop_excess_property_checking_through_a_widened_binding() {
    let session = TestSession::single(
        r#"
const source = { name: "Ada", extra: 1 };
const target: { name: string } = source;
const direct: { name: string } = { name: "Ada", extra: 1 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const source: { name: string; extra: int64 } = { name: "Ada", extra: 1 };
const target: { name: string } = source;
const direct: { name: string } = { name: "Ada", extra: 1 };

=== dir ===
const source = { name: "Ada", extra: 1 };
/// @type.symbol symbol=source source=source type={ name: string; extra: int64 }
/// @resolution.pattern source=source kind=binding target=source
/// @type.node source={ name: "Ada", extra: 1 } type={ name: string; extra: int64 }
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=1 type=1

const target: { name: string } = source;
/// @type.symbol symbol=target source=target type={ name: string }
/// @resolution.pattern source=target kind=binding target=target
/// @type.symbol symbol=name#1 source="name: string" type=string
/// @type.node source=source type={ name: string; extra: int64 }
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="immutable"
/// @resolution.access source=source root=source

const direct: { name: string } = { name: "Ada", extra: 1 };
/// @type.symbol symbol=direct source=direct type={ name: string }
/// @resolution.pattern source=direct kind=binding target=direct
/// @type.symbol symbol=name#2 source="name: string" type=string
/// @type.node source={ name: "Ada", extra: 1 } type={ name: string; extra: int64 }
/// @type.node source="\"Ada\"" type="Ada"
/// @type.node source=1 type=1
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ name: string; extra: int64 }' is not assignable to type '{ name: string }'"
/// @diagnostic.label line=3 column=34 span="source" line_source="const target: { name: string } = source;"
/// @diagnostic.related line=3 column=15 span="{ name: string }" line_source="const target: { name: string } = source;" message="expected due to this annotation"
/// @diagnostic.note message="'{ name: string }' stores its exact object type, declare an interface to accept structurally wider values"
/// @diagnostic.error id=excess-property message="unknown property 'extra' in object literal for type '{ name: string }'"
/// @diagnostic.label line=4 column=34 span="{ name: \"Ada\", extra: 1 }" line_source="const direct: { name: string } = { name: \"Ada\", extra: 1 };"
/// @diagnostic.related line=4 column=15 span="{ name: string }" line_source="const direct: { name: string } = { name: \"Ada\", extra: 1 };" message="expected due to this annotation"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}
