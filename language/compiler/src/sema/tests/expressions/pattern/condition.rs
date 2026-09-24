use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_condition_binding_shadowing_type() {
    let session = TestSession::single(
        r#"
struct Cancelled {
    reason: int32;
}

function read(value: Cancelled | int32): int32 {
    if (let Cancelled = value) {
        return 0;
    }

    1
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Cancelled {
    reason: int32;
}

function read(value: Cancelled | int32): int32 {
    if (let Cancelled: Cancelled | int32 = value) {
        return 0;
    }

    1
}

=== dir ===
struct Cancelled {
/// @type.symbol symbol=Cancelled type=Cancelled
/// @definition.struct symbol=Cancelled
/// @definition.field symbol=Cancelled.reason source="reason: int32" key=reason type=int32

    reason: int32;
    /// @type.symbol symbol=Cancelled.reason source="reason: int32" type=int32

}

function read(value: Cancelled | int32): int32 {
/// @type.symbol symbol=read type=(Cancelled | int32) => int32
/// @type.symbol symbol=read.value source="value: Cancelled | int32" type=Cancelled | int32
/// @resolution.name source=Cancelled target=Cancelled

    if (let Cancelled = value) {
    /// @type.symbol symbol=read.Cancelled source=Cancelled type=Cancelled | int32
    /// @resolution.pattern source=Cancelled kind=binding target=read.Cancelled
    /// @resolution.name source=value target=read.value
    /// @resolution.access source=value root=read.value

        return 0;
    }

    1
}
"#,
        r#"
/// @diagnostic.error id=pattern-shadows-type message="bare pattern 'Cancelled' binds a new variable that shadows a type"
/// @diagnostic.label line=7 column=13 span="Cancelled" line_source="if (let Cancelled = value) {"
/// @diagnostic.help message="match values of the type with a nominal pattern like 'Cancelled { }'"
"#,
    );
}

#[test]
fn test_if_let_pattern_binds_then_branch() {
    let session = TestSession::single(
        r#"
declare const pair: (int32, string) | null;

if (let (count, label) = pair) {
    count satisfies int32;
    label satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: (int32, string) | null;

if (let (count, label) = pair) {
    count satisfies int32;
    label satisfies string;
}

=== dir ===
declare const pair: (int32, string) | null;
/// @type.symbol symbol=pair source=pair type=(int32, string) | null
/// @resolution.pattern source=pair kind=binding target=pair

if (let (count, label) = pair) {
/// @resolution.pattern source=(count, label) kind=tuple fields=(count, label)
/// @type.symbol symbol=count source=count type=int32
/// @resolution.pattern source=count kind=binding target=count
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source=pair type=(int32, string) | null
/// @resolution.name source=pair target=pair
/// @resolution.access source=pair root=pair

    count satisfies int32;
    /// @type.node source="count satisfies int32" type=int32
    /// @type.node source=count type=int32
    /// @resolution.name source=count target=count
    /// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=count root=count

    label satisfies string;
    /// @type.node source="label satisfies string" type=string
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label
    /// @resolution.place source=label placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=label root=label

}
"#,
    );
}

#[test]
fn test_if_let_object_pattern_binds_then_branch() {
    let session = TestSession::single(
        r#"
declare const config: { enabled: boolean; retries: int32 } | null;

if (let { enabled, retries } = config) {
    enabled satisfies boolean;
    retries satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const config: { enabled: boolean; retries: int32 } | null;

if (let { enabled, retries } = config) {
    enabled satisfies boolean;
    retries satisfies int32;
}

=== dir ===
declare const config: { enabled: boolean; retries: int32 } | null;
/// @type.symbol symbol=config source=config type={ enabled: boolean; retries: int32 } | null
/// @resolution.pattern source=config kind=binding target=config
/// @type.symbol symbol=enabled#1 source="enabled: boolean" type=boolean
/// @type.symbol symbol=retries#1 source="retries: int32" type=int32

if (let { enabled, retries } = config) {
/// @resolution.pattern source={ enabled, retries } kind=object fields={ enabled, retries }
/// @type.symbol symbol=enabled#2 source=enabled type=boolean
/// @type.symbol symbol=retries#2 source=retries type=int32
/// @type.node source=config type={ enabled: boolean; retries: int32 } | null
/// @resolution.name source=config target=config
/// @resolution.access source=config root=config

    enabled satisfies boolean;
    /// @type.node source="enabled satisfies boolean" type=boolean
    /// @type.node source=enabled type=boolean
    /// @resolution.name source=enabled target=enabled#2
    /// @resolution.place source=enabled placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=enabled root=enabled#2

    retries satisfies int32;
    /// @type.node source="retries satisfies int32" type=int32
    /// @type.node source=retries type=int32
    /// @resolution.name source=retries target=retries#2
    /// @resolution.place source=retries placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=retries root=retries#2

}
"#,
    );
}

#[test]
fn test_if_condition_chain_binds_later_operands() {
    let session = TestSession::single(
        r#"
declare const ready: boolean;
declare const pair: (int32, string) | null;

if (ready && let (count, label) = pair && count > 0) {
    label satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const ready: boolean;
declare const pair: (int32, string) | null;

if (ready && let (count, label) = pair && count > 0) {
    label satisfies string;
}

=== dir ===
declare const ready: boolean;
/// @type.symbol symbol=ready source=ready type=boolean
/// @resolution.pattern source=ready kind=binding target=ready

declare const pair: (int32, string) | null;
/// @type.symbol symbol=pair source=pair type=(int32, string) | null
/// @resolution.pattern source=pair kind=binding target=pair

if (ready && let (count, label) = pair && count > 0) {
/// @type.node source=ready type=boolean
/// @resolution.name source=ready target=ready
/// @resolution.place source=ready placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ready root=ready
/// @resolution.pattern source=(count, label) kind=tuple fields=(count, label)
/// @type.symbol symbol=count source=count type=int32
/// @resolution.pattern source=count kind=binding target=count
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source=pair type=(int32, string) | null
/// @resolution.name source=pair target=pair
/// @resolution.access source=pair root=pair
/// @type.node source="count > 0" type=boolean
/// @type.node source=count type=int32
/// @resolution.name source=count target=count
/// @resolution.operator source="count > 0" type=boolean operator=">" kind=builtin operands=[count as int32 families=(integer), 0 as int32 families=(integer)]
/// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=count root=count
/// @type.node source=0 type=0

    label satisfies string;
    /// @type.node source="label satisfies string" type=string
    /// @type.node source=label type=string
    /// @resolution.name source=label target=label
    /// @resolution.place source=label placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=label root=label

}
"#,
    );
}

#[test]
fn test_if_let_literal_pattern_narrows_source() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

if (let "ready" = status) {
    status satisfies "ready";
} else {
    status satisfies "error";
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

if (let "ready" = status) {
    status satisfies "ready";
} else {
    status satisfies "error";
}

=== dir ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"
/// @resolution.pattern source=status kind=binding target=status

if (let "ready" = status) {
/// @type.node source="\"ready\"" type="ready"
/// @resolution.pattern source="\"ready\"" kind=literal value="ready"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status
/// @resolution.access source=status root=status

    status satisfies "ready";
    /// @type.node source="status satisfies \"ready\"" type="ready"
    /// @type.node source=status type="ready"
    /// @resolution.name source=status target=status
    /// @resolution.place source=status placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=status root=status
    /// @resolution.narrowing source=status union="ready" | "error" arms="ready"

} else {
    status satisfies "error";
    /// @type.node source="status satisfies \"error\"" type="error"
    /// @type.node source=status type="error"
    /// @resolution.name source=status target=status
    /// @resolution.place source=status placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=status root=status
    /// @resolution.narrowing source=status union="ready" | "error" arms="error"

}
"#,
    );
}

#[test]
fn test_if_let_union_pattern_narrows_covered_literals() {
    let session = TestSession::single(
        r#"
declare const value: 1 | 2 | 3;

if (let 1 | 2 = value) {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: 1 | 2 | 3;

if (let 1 | 2 = value) {
    value satisfies 1 | 2;
} else {
    value satisfies 3;
}

=== dir ===
declare const value: 1 | 2 | 3;
/// @type.symbol symbol=value source=value type=1 | 2 | 3
/// @resolution.pattern source=value kind=binding target=value

if (let 1 | 2 = value) {
/// @type.node source=1 type=1
/// @resolution.pattern source="1 | 2" kind=union patterns=[pattern, pattern]
/// @resolution.pattern source=1 kind=literal value=1
/// @type.node source=2 type=2
/// @resolution.pattern source=2 kind=literal value=2
/// @type.node source=value type=1 | 2 | 3
/// @resolution.name source=value target=value
/// @resolution.access source=value root=value

    value satisfies 1 | 2;
    /// @type.node source="value satisfies 1 | 2" type=1 | 2
    /// @type.node source=value type=1 | 2
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union=1 | 2 | 3 arms=1 | 2

} else {
    value satisfies 3;
    /// @type.node source="value satisfies 3" type=3
    /// @type.node source=value type=3
    /// @resolution.name source=value target=value
    /// @resolution.place source=value placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=value root=value
    /// @resolution.narrowing source=value union=1 | 2 | 3 arms=3

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_binding_after_boolean_operand() {
    let session = TestSession::single(
        r#"
declare const ready: boolean;
declare const user: { name: string } | null;

if (ready && let { name } = user) {
    name satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const ready: boolean;
declare const user: { name: string } | null;

if (ready && let { name } = user) {
    name satisfies string;
}

=== dir ===
declare const ready: boolean;
/// @type.symbol symbol=ready source=ready type=boolean
/// @resolution.pattern source=ready kind=binding target=ready

declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#1 source="name: string" type=string

if (ready && let { name } = user) {
/// @type.node source=ready type=boolean
/// @resolution.name source=ready target=ready
/// @resolution.place source=ready placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ready root=ready
/// @resolution.pattern source={ name } kind=object fields={ name }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_boolean_operand_after_binding() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;

if (let { name } = user && name.length > 0) {
    name satisfies string;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;

if (let { name } = user && name.length > 0) {
    name satisfies string;
}

=== dir ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#1 source="name: string" type=string

if (let { name } = user && name.length > 0) {
/// @resolution.pattern source={ name } kind=object fields={ name }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user
/// @type.node source="name.length > 0" type=boolean
/// @type.node source=name type=string
/// @type.node source=name.length type=isize
/// @resolution.name source=name target=name#2
/// @resolution.member source=name.length receiver=string type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.operator source="name.length > 0" type=boolean operator=">" kind=builtin operands=[name.length as isize families=(integer), 0 as isize families=(integer)]
/// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=name root=name#2
/// @generic.instantiation id="length<\"managed\" & \"local\">" template=length arguments=("managed" & "local")
/// @generic.instance id="length<\"bound0\" & \"local\">" template=length arguments=("bound0" & "local")
/// @type.node source=0 type=0

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

}
"#,
    );
}

#[test]
fn test_condition_chain_allows_multiple_bindings() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;
declare const point: { x: int32 } | null;

if (let { name } = user && let { x } = point) {
    name satisfies string;
    x satisfies int32;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;
declare const point: { x: int32 } | null;

if (let { name } = user && let { x } = point) {
    name satisfies string;
    x satisfies int32;
}

=== dir ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#1 source="name: string" type=string

declare const point: { x: int32 } | null;
/// @type.symbol symbol=point source=point type={ x: int32 } | null
/// @resolution.pattern source=point kind=binding target=point
/// @type.symbol symbol=x#1 source="x: int32" type=int32

if (let { name } = user && let { x } = point) {
/// @resolution.pattern source={ name } kind=object fields={ name }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user
/// @resolution.pattern source={ x } kind=object fields={ x }
/// @type.symbol symbol=x#2 source=x type=int32
/// @type.node source=point type={ x: int32 } | null
/// @resolution.name source=point target=point
/// @resolution.access source=point root=point

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

    x satisfies int32;
    /// @type.node source="x satisfies int32" type=int32
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x#2
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=x#2

}
"#,
    );
}

#[test]
fn test_condition_chain_else_branch_cannot_see_binding() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;

if (let { name } = user) {
    name satisfies string;
} else {
    name;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;

if (let { name } = user) {
    name satisfies string;
} else {
    name;
}

=== dir ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#1 source="name: string" type=string

if (let { name } = user) {
/// @resolution.pattern source={ name } kind=object fields={ name }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

} else {
    name;
    /// @type.node source=name type=<error>
    /// @resolution.unresolved source=name path=name

}
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'name'"
/// @diagnostic.label line=7 column=5 span="name" line_source="name;"
"#,
    );
}

/// Bind while-condition names through later operands and the loop body only.
#[test]
fn test_while_condition_binds_chain_and_body() {
    let session = TestSession::single(
        r#"
declare const user: { name: string } | null;

while (let { name } = user && name.length > 0) {
    name satisfies string;
    break;
}

name;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const user: { name: string } | null;

while (let { name } = user && name.length > 0) {
    name satisfies string;
    break;
}

name;

=== dir ===
declare const user: { name: string } | null;
/// @type.symbol symbol=user source=user type={ name: string } | null
/// @resolution.pattern source=user kind=binding target=user
/// @type.symbol symbol=name#1 source="name: string" type=string

while (let { name } = user && name.length > 0) {
/// @resolution.pattern source={ name } kind=object fields={ name }
/// @type.symbol symbol=name#2 source=name type=string
/// @type.node source=user type={ name: string } | null
/// @resolution.name source=user target=user
/// @resolution.access source=user root=user
/// @type.node source="name.length > 0" type=boolean
/// @type.node source=name type=string
/// @type.node source=name.length type=isize
/// @resolution.name source=name target=name#2
/// @resolution.member source=name.length receiver=string type=isize kind=call target="length(parameters=(), arguments=(), return=isize, regions=(\"managed\" & \"local\"))"
/// @resolution.operator source="name.length > 0" type=boolean operator=">" kind=builtin operands=[name.length as isize families=(integer), 0 as isize families=(integer)]
/// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=name root=name#2
/// @generic.instantiation id="length<\"managed\" & \"local\">" template=length arguments=("managed" & "local")
/// @type.node source=0 type=0

    name satisfies string;
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name#2
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name#2

    break;
    /// @type.node source=break type=never
    /// @resolution.transfer source=break target=while

}

name;
/// @type.node source=name type=<error>
/// @resolution.unresolved source=name path=name
"#,
        r#"
/// @diagnostic.error id=unresolved-reference message="cannot find 'name'"
/// @diagnostic.label line=9 column=1 span="name" line_source="name;"
"#,
    );
}

#[test]
fn test_condition_chain_without_binding_stays_boolean_expression() {
    let session = TestSession::single(
        r#"
declare const left: boolean;
declare const right: boolean;

if (left && right) {
    left satisfies boolean;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const left: boolean;
declare const right: boolean;

if (left && right) {
    left satisfies boolean;
}

=== dir ===
declare const left: boolean;
/// @type.symbol symbol=left source=left type=boolean
/// @resolution.pattern source=left kind=binding target=left

declare const right: boolean;
/// @type.symbol symbol=right source=right type=boolean
/// @resolution.pattern source=right kind=binding target=right

if (left && right) {
/// @type.node source="left && right" type=boolean
/// @type.node source=left type=boolean
/// @resolution.name source=left target=left
/// @resolution.operator source="left && right" type=boolean operator="&&" kind=builtin operands=[left as boolean families=(boolean), right as boolean families=(boolean)]
/// @resolution.place source=left placement="local" lifetime="static" access="immutable"
/// @resolution.access source=left root=left
/// @type.node source=right type=boolean
/// @resolution.name source=right target=right
/// @resolution.place source=right placement="local" lifetime="static" access="immutable"
/// @resolution.access source=right root=right

    left satisfies boolean;
    /// @type.node source="left satisfies boolean" type=boolean
    /// @type.node source=left type=boolean
    /// @resolution.name source=left target=left
    /// @resolution.place source=left placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=left root=left

}
"#,
    );
}
