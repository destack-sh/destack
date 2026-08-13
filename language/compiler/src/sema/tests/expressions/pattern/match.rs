use crate::tests::{DirRows, TestSession};

#[test]
fn test_switch_numeric_literal_checks() {
    let session = TestSession::single(
        r#"
switch (1) {
    case 1: debugger;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
switch (1) {
    case 1:
        debugger;
}

=== checked ===
switch (1) {
/// @type.node source=1 type=1

    case 1: debugger;
    /// @resolution.operator source="case 1: debugger;" type=boolean operator="===" kind=builtin operands=[1 as float64 families=(float), 1 as float64 families=(float)]
    /// @type.node source=1 type=1
    /// @type.node source=debugger type=void

}
"#,
    );
}

#[test]
fn test_switch_string_literal_checks() {
    let session = TestSession::single(
        r#"
switch ("ready") {
    case "ready": debugger;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
switch ("ready") {
    case "ready":
        debugger;
}

=== checked ===
switch ("ready") {
/// @type.node source="\"ready\"" type="ready"

    case "ready": debugger;
    /// @resolution.operator source="case \"ready\": debugger;" type=boolean operator="===" kind=builtin operands=["ready" as "ready" families=(string), "ready" as "ready" families=(string)]
    /// @type.node source="\"ready\"" type="ready"
    /// @type.node source=debugger type=void

}
"#,
    );
}

#[test]
fn test_switch_bigint_literal_checks() {
    let session = TestSession::single(
        r#"
switch (1n) {
    case 1n: debugger;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
switch (1n) {
    case 1n:
        debugger;
}

=== checked ===
switch (1n) {
/// @type.node source=1n type=1n

    case 1n: debugger;
    /// @resolution.operator source="case 1n: debugger;" type=boolean operator="===" kind=builtin operands=[1n as bigint families=(bigint), 1n as bigint families=(bigint)]
    /// @type.node source=1n type=1n
    /// @type.node source=debugger type=void

}
"#,
    );
}

#[test]
fn test_switch_evaluates_selectors_before_default() {
    let session = TestSession::single(
        r#"
declare const selected: int32;
let observed: int32;

switch (selected) {
    default: break;
    case (observed = 1): break;
}

observed satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const selected: int32;
let observed: int32;

switch (selected) {
    default:
        break;
    case (observed = 1):
        break;
}

observed satisfies int32;

=== checked ===
declare const selected: int32;
/// @type.symbol symbol=selected source=selected type=int32
/// @resolution.pattern source=selected kind=binding target=selected

let observed: int32;
/// @type.symbol symbol=observed source=observed type=int32
/// @resolution.pattern source=observed kind=binding target=observed

switch (selected) {
/// @type.node source=selected type=int32
/// @resolution.name source=selected target=selected
/// @resolution.place source=selected placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=selected root=selected

    default: break;
    /// @type.node source=break type=never

    case (observed = 1): break;
    /// @resolution.operator source="case (observed = 1): break;" type=boolean operator="===" kind=builtin operands=[selected as int32 families=(integer), observed = 1 as int32 families=(integer)]
    /// @type.node source="observed = 1" type=1
    /// @type.node source=observed type=int32
    /// @resolution.name source=observed target=observed
    /// @resolution.pattern.assign source=observed kind=place
    /// @resolution.access source=observed root=observed
    /// @resolution.assignment source=observed write=binding(observed) type=int32
    /// @type.node source=1 type=1
    /// @type.node source=break type=never

}

observed satisfies int32;
/// @type.node source="observed satisfies int32" type=int32
/// @type.node source=observed type=int32
/// @resolution.name source=observed target=observed
/// @resolution.place source=observed placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=observed root=observed
"#,
    );
}

#[test]
fn test_switch_with_default_and_returning_cases_does_not_complete() {
    let session = TestSession::single(
        r#"
function classify(value: int32): int32 {
    switch (value) {
        case 0: return 0;
        default: return 1;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function classify(value: int32): int32 {
    switch (value) {
        case 0:
            return 0;
        default:
            return 1;
    }
}

=== checked ===
function classify(value: int32): int32 {
/// @type.symbol symbol=classify type=(int32) => int32
/// @type.symbol symbol=classify.value source="value: int32" type=int32

    switch (value) {
    /// @type.node type=never
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=classify.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=classify.value

        case 0: return 0;
        /// @resolution.operator source="case 0: return 0;" type=boolean operator="===" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
        /// @type.node source=0 type=0
        /// @type.node source=0 type=0

        default: return 1;
        /// @type.node source=1 type=1

    }
}
"#,
    );
}

#[test]
fn test_match_omits_statically_absent_arm() {
    let session = TestSession::single(
        r#"
const value = match (true) {
    @if(false)
    false => missingValue
    _ => 1
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 1 = match (true) {
    @if(false)
    false => missingValue
    _ => 1
};

=== checked ===
const value = match (true) {
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @type.node type=1
/// @type.node source=true type=true

    @if(false)
    false => missingValue
    _ => 1
    /// @resolution.pattern source=_ kind=wildcard
    /// @type.node source=1 type=1

};
"#,
    );
}

#[test]
fn test_switch_omits_statically_absent_case() {
    let session = TestSession::single(
        r#"
switch (0) {
    @if(false)
    case missingValue: missingCall();
    default: debugger;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
switch (0) {
    @if(false)
    case missingValue:
        missingCall();
    default:
        debugger;
}

=== checked ===
switch (0) {
/// @type.node source=0 type=0

    @if(false)
    case missingValue: missingCall();
    default: debugger;
    /// @type.node source=debugger type=void

}
"#,
    );
}

#[test]
fn test_switch_narrows_enum_members() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read = 1,
    Write = 2,
    Execute = 3,
}

declare const mode: Mode;
switch (mode) {
    case Mode.Read:
        mode;
        break;
    case Mode.Write:
        mode;
        break;
    default:
        mode;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Mode {
    Read = 1,
    Write = 2,
    Execute = 3,
}

declare const mode: Mode;
switch (mode) {
    case Mode.Read:
        mode;
        break;
    case Mode.Write:
        mode;
        break;
    default:
        mode;
}

=== checked ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Execute source="Execute = 3" key=Execute value=3
/// @definition.variant symbol=Mode.Read source="Read = 1" key=Read value=1
/// @definition.variant symbol=Mode.Write source="Write = 2" key=Write value=2

    Read = 1,
    /// @type.symbol symbol=Mode.Read source="Read = 1" type=Mode.Read
    /// @type.node source=1 type=1

    Write = 2,
    /// @type.symbol symbol=Mode.Write source="Write = 2" type=Mode.Write
    /// @type.node source=2 type=2

    Execute = 3,
    /// @type.symbol symbol=Mode.Execute source="Execute = 3" type=Mode.Execute
    /// @type.node source=3 type=3

}

declare const mode: Mode;
/// @type.symbol symbol=mode source=mode type=Mode
/// @resolution.pattern source=mode kind=binding target=mode
/// @resolution.name source=Mode target=Mode

switch (mode) {
/// @type.node source=mode type=Mode
/// @resolution.name source=mode target=mode
/// @resolution.place source=mode placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=mode root=mode

    case Mode.Read:
    /// @resolution.operator type=boolean operator="===" kind=builtin operands=[mode as Mode families=(Mode), Mode.Read as Mode.Read families=(Mode)]
    /// @type.node source=Mode type=Mode
    /// @type.node source=Mode.Read type=Mode.Read
    /// @resolution.name source=Mode target=Mode
    /// @resolution.member source=Mode.Read receiver=Mode type=Mode.Read kind=symbol target_receiver=Mode target=Mode.Read

        mode;
        /// @type.node source=mode type=Mode.Read
        /// @resolution.name source=mode target=mode
        /// @resolution.place source=mode placement="local" lifetime="static" access="exclusive"
        /// @resolution.access source=mode root=mode

        break;
        /// @type.node source=break type=never

    case Mode.Write:
    /// @resolution.operator type=boolean operator="===" kind=builtin operands=[mode as Mode families=(Mode), Mode.Write as Mode.Write families=(Mode)]
    /// @type.node source=Mode type=Mode
    /// @type.node source=Mode.Write type=Mode.Write
    /// @resolution.name source=Mode target=Mode
    /// @resolution.member source=Mode.Write receiver=Mode type=Mode.Write kind=symbol target_receiver=Mode target=Mode.Write

        mode;
        /// @type.node source=mode type=Mode.Write
        /// @resolution.name source=mode target=mode
        /// @resolution.place source=mode placement="local" lifetime="static" access="exclusive"
        /// @resolution.access source=mode root=mode

        break;
        /// @type.node source=break type=never

    default:
        mode;
        /// @type.node source=mode type=Mode.Execute
        /// @resolution.name source=mode target=mode
        /// @resolution.place source=mode placement="local" lifetime="static" access="exclusive"
        /// @resolution.access source=mode root=mode

}
"#,
    );
}

#[test]
fn test_switch_narrows_singleton_newtypes() {
    let session = TestSession::single(
        r#"
newtype Ready = "ready";
newtype Pending = "pending";

declare const state: Ready | Pending;
switch (state) {
    case Ready("ready"):
        state;
        break;
    default:
        state;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
newtype Ready = "ready";
newtype Pending = "pending";

declare const state: Ready | Pending;
switch (state) {
    case Ready("ready") as Ready | Pending:
        state;
        break;
    default:
        state;
}

=== checked ===
newtype Ready = "ready";
/// @type.symbol symbol=Ready source="newtype Ready = \"ready\"" type=Ready
/// @definition.newtype symbol=Ready source="newtype Ready = \"ready\"" backing="ready" constructors=[("ready") => Ready]

newtype Pending = "pending";
/// @type.symbol symbol=Pending source="newtype Pending = \"pending\"" type=Pending
/// @definition.newtype symbol=Pending source="newtype Pending = \"pending\"" backing="pending" constructors=[("pending") => Pending]

declare const state: Ready | Pending;
/// @type.symbol symbol=state source=state type=Ready | Pending
/// @resolution.pattern source=state kind=binding target=state
/// @resolution.name source=Ready target=Ready
/// @resolution.name source=Pending target=Pending

switch (state) {
/// @type.node source=state type=Ready | Pending
/// @resolution.name source=state target=state
/// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=state root=state

    case Ready("ready"):
    /// @resolution.operator type=boolean operator="===" kind=builtin operands=[state as Ready | Pending families=(string), Ready("ready") as Ready | Pending families=(string)]
    /// @type.node source="Ready(\"ready\")" type=Ready
    /// @type.node source=Ready type=Ready
    /// @resolution.name source=Ready target=Ready
    /// @resolution.construct source="Ready(\"ready\")" parameters=("ready") arguments=(provided("ready") as "ready") return=Ready kind=newtype target=Ready backing="ready"
    /// @type.node source="\"ready\"" type="ready"

        state;
        /// @type.node source=state type=Ready
        /// @resolution.name source=state target=state
        /// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
        /// @resolution.access source=state root=state

        break;
        /// @type.node source=break type=never

    default:
        state;
        /// @type.node source=state type=Pending
        /// @resolution.name source=state target=state
        /// @resolution.place source=state placement="local" lifetime="static" access="exclusive"
        /// @resolution.access source=state root=state

}
"#,
    );
}

#[test]
fn test_match_boolean_is_exhaustive() {
    let session = TestSession::single(
        r#"
declare const value: boolean;

const label = match (value) {
    true => "yes"
    false => "no"
};

label satisfies "yes" | "no";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: boolean;

const label: "yes" | "no" = match (value) {
    true => "yes"
    false => "no"
};

label satisfies "yes" | "no";

=== checked ===
declare const value: boolean;
/// @type.symbol symbol=value source=value type=boolean
/// @resolution.pattern source=value kind=binding target=value

const label = match (value) {
/// @type.symbol symbol=label source=label type="yes" | "no"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="yes" | "no"
/// @type.node source=value type=boolean
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

    true => "yes"
    /// @type.node source=true type=true
    /// @resolution.pattern source=true kind=literal value=true
    /// @type.node source="\"yes\"" type="yes"

    false => "no"
    /// @type.node source=false type=false
    /// @resolution.pattern source=false kind=literal value=false
    /// @type.node source="\"no\"" type="no"

};

label satisfies "yes" | "no";
/// @type.node source="label satisfies \"yes\" | \"no\"" type="yes" | "no"
/// @type.node source=label type="yes" | "no"
/// @resolution.name source=label target=label
/// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=label root=label
"#,
    );
}

#[test]
fn test_empty_match_is_exhaustive_for_never() {
    let session = TestSession::single(
        r#"
declare const value: never;

const result = match (value) {};

result satisfies never;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: never;

const result: never = match (value) {};

result satisfies never;

=== checked ===
declare const value: never;
/// @type.symbol symbol=value source=value type=never
/// @resolution.pattern source=value kind=binding target=value

const result = match (value) {};
/// @type.symbol symbol=result source=result type=never
/// @resolution.pattern source=result kind=binding target=result
/// @type.node source="match (value) {}" type=never
/// @type.node source=value type=never
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

result satisfies never;
/// @type.node source="result satisfies never" type=never
/// @type.node source=result type=never
/// @resolution.name source=result target=result
/// @resolution.place source=result placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=result root=result
"#,
    );
}

#[test]
fn test_match_literal_union_is_exhaustive() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
    "error" => "stop"
};

label satisfies "go" | "stop";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "stop" = match (status) {
    "ready" => "go"
    "error" => "stop"
};

label satisfies "go" | "stop";

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"
/// @resolution.pattern source=status kind=binding target=status

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "stop"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="go" | "stop"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

    "error" => "stop"
    /// @type.node source="\"error\"" type="error"
    /// @resolution.pattern source="\"error\"" kind=literal value="error"
    /// @type.node source="\"stop\"" type="stop"

};

label satisfies "go" | "stop";
/// @type.node source="label satisfies \"go\" | \"stop\"" type="go" | "stop"
/// @type.node source=label type="go" | "stop"
/// @resolution.name source=label target=label
/// @resolution.place source=label placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=label root=label
"#,
    );
}

#[test]
fn test_match_reports_missing_literal_union_member() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" = match (status) {
    "ready" => "go"
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"
/// @resolution.pattern source=status kind=binding target=status

const label = match (status) {
/// @type.symbol symbol=label source=label type="go"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="go"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

};
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '\"error\"' is not covered"
/// @diagnostic.label line=4 column=15 span="match (status) {\n    \"ready\" => \"go\"\n}" line_source="const label = match (status) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}

#[test]
fn test_match_guard_does_not_prove_exhaustiveness() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" if (true) => "go"
    "error" => "stop"
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "stop" = match (status) {
    "ready" if (true) => "go"
    "error" => "stop"
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"
/// @resolution.pattern source=status kind=binding target=status

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "stop"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="go" | "stop"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    "ready" if (true) => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=true type=true
    /// @type.node source="\"go\"" type="go"

    "error" => "stop"
    /// @type.node source="\"error\"" type="error"
    /// @resolution.pattern source="\"error\"" kind=literal value="error"
    /// @type.node source="\"stop\"" type="stop"

};
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '\"ready\"' is not covered"
/// @diagnostic.label line=4 column=15 span="match (status) {\n    \"ready\" if (true) => \"go\"\n    \"error\" => \"stop\"\n}" line_source="const label = match (status) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
/// @diagnostic.warning id=constant-condition message="condition is always true"
/// @diagnostic.label line=5 column=17 span="true" line_source="\"ready\" if (true) => \"go\""
"#,
    );
}

#[test]
fn test_match_guard_can_use_pattern_bindings() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: int32 };

const result = match (point) {
    { x, y } if (x == x) => y
    _ => 0
};

result satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: int32 };

const result: int32 = match (point) {
    { x, y } if (x == x) => y
    _ => 0
};

result satisfies int32;

=== checked ===
declare const point: { x: int32; y: int32 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }
/// @resolution.pattern source=point kind=binding target=point

const result = match (point) {
/// @type.symbol symbol=result source=result type=int32
/// @resolution.pattern source=result kind=binding target=result
/// @type.node type=int32
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point
/// @resolution.place source=point placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=point root=point

    { x, y } if (x == x) => y
    /// @resolution.pattern source={ x, y } kind=object fields={ x, y }
    /// @type.symbol symbol=x#2 source=x type=int32
    /// @type.symbol symbol=y#2 source=y type=int32
    /// @type.node source="x == x" type=boolean
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x#2
    /// @resolution.operator source="x == x" type=boolean operator="==" kind=builtin operands=[x as int32 families=(integer), x as int32 families=(integer)]
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=x#2
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x#2
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=x#2
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y#2
    /// @resolution.place source=y placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=y root=y#2

    _ => 0
    /// @resolution.pattern source=_ kind=wildcard
    /// @type.node source=0 type=0

};

result satisfies int32;
/// @type.node source="result satisfies int32" type=int32
/// @type.node source=result type=int32
/// @resolution.name source=result target=result
/// @resolution.place source=result placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=result root=result
"#,
    );
}

#[test]
fn test_match_object_pattern_binds_fields() {
    let session = TestSession::single(
        r#"
declare const config: { enabled: boolean; retries: int32 };

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const config: { enabled: boolean; retries: int32 };

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}

=== checked ===
declare const config: { enabled: boolean; retries: int32 };
/// @type.symbol symbol=config source=config type={ enabled: boolean; retries: int32 }
/// @resolution.pattern source=config kind=binding target=config

match (config) {
/// @type.node source=config type={ enabled: boolean; retries: int32 }
/// @resolution.name source=config target=config
/// @resolution.place source=config placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=config root=config

    { enabled, retries } => {
    /// @resolution.pattern source={ enabled, retries } kind=object fields={ enabled, retries }
    /// @type.symbol symbol=enabled#2 source=enabled type=boolean
    /// @type.symbol symbol=retries#2 source=retries type=int32

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
}
"#,
    );
}

#[test]
fn test_match_nested_patterns_bind_leaf_values() {
    let session = TestSession::single(
        r#"
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };

match (packet) {
    {
        point: { x, y },
        labels: [first, second],
    } => {
        x satisfies int32;
        y satisfies int32;
        first satisfies string;
        second satisfies string;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };

match (packet) {
    {
        point: { x, y },
        labels: [first, second],
    } => {
        x satisfies int32;
        y satisfies int32;
        first satisfies string;
        second satisfies string;
    }
}

=== checked ===
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };
/// @type.symbol symbol=packet source=packet type={ point: { x: int32; y: int32 }; labels: FixedArray<string, 2> }
/// @resolution.pattern source=packet kind=binding target=packet

match (packet) {
/// @type.node source=packet type={ point: { x: int32; y: int32 }; labels: FixedArray<string, 2> }
/// @resolution.name source=packet target=packet
/// @resolution.place source=packet placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=packet root=packet

    {
    /// @resolution.pattern kind=object fields={ point: pattern, labels: pattern }

        point: { x, y },
        /// @resolution.pattern source={ x, y } kind=object fields={ x, y }
        /// @type.symbol symbol=x#2 source=x type=int32
        /// @type.symbol symbol=y#2 source=y type=int32

        labels: [first, second],
        /// @resolution.pattern source=[first, second] kind=sequence element=string arity=2 fields=(first, second)
        /// @type.symbol symbol=first source=first type=string
        /// @resolution.pattern source=first kind=binding target=first
        /// @type.symbol symbol=second source=second type=string
        /// @resolution.pattern source=second kind=binding target=second

    } => {
        x satisfies int32;
        /// @type.node source="x satisfies int32" type=int32
        /// @type.node source=x type=int32
        /// @resolution.name source=x target=x#2
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=x#2

        y satisfies int32;
        /// @type.node source="y satisfies int32" type=int32
        /// @type.node source=y type=int32
        /// @resolution.name source=y target=y#2
        /// @resolution.place source=y placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=y root=y#2

        first satisfies string;
        /// @type.node source="first satisfies string" type=string
        /// @type.node source=first type=string
        /// @resolution.name source=first target=first
        /// @resolution.place source=first placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=first root=first

        second satisfies string;
        /// @type.node source="second satisfies string" type=string
        /// @type.node source=second type=string
        /// @resolution.name source=second target=second
        /// @resolution.place source=second placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=second root=second

    }
}
"#,
    );
}

#[test]
fn test_match_nominal_object_pattern_binds_class_fields() {
    let session = TestSession::single(
        r#"
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

declare const user: User;

match (user) {
    User { name } => name satisfies string
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;

    constructor(name: string): this {
        this.name = name;
    }
}

declare const user: User;

match (user) {
    User { name } => name satisfies string
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(string) => this

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=User.constructor type=(string) => this
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=User, target=field(receiver=User, target=User.name, type=string), type=string" type=string
        /// @type.node source=name type=string
        /// @resolution.name source=name target=User.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=User.constructor.name

    }
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

match (user) {
/// @type.node type=string
/// @type.node source=user type=User
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=user root=user

    User { name } => name satisfies string
    /// @resolution.name source=User target=User
    /// @resolution.pattern source="User { name }" kind=nominal_object target=User fields={ User.name }
    /// @type.symbol symbol=name source=name type=string
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=name

}
"#,
    );
}

#[test]
fn test_match_rest_pattern_binds_remaining_sequence() {
    let session = TestSession::single(
        r#"
declare const values: int32[];

match (values) {
    [head, ...tail] => {
        head satisfies int32;
        tail satisfies ^int32[];
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: int32[];

match (values) {
    [head, ...tail] => {
        head satisfies int32;
        tail satisfies ^int32[];
    }
}

=== checked ===
declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values

match (values) {
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values

    [head, ...tail] => {
    /// @resolution.pattern source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
    /// @type.symbol symbol=head source=head type=int32
    /// @resolution.pattern source=head kind=binding target=head
    /// @type.symbol symbol=tail source=tail type=Owned<Array<int32>>
    /// @resolution.pattern source=tail kind=binding target=tail

        head satisfies int32;
        /// @type.node source="head satisfies int32" type=int32
        /// @type.node source=head type=int32
        /// @resolution.name source=head target=head
        /// @resolution.place source=head placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=head root=head

        tail satisfies ^int32[];
        /// @type.node source="tail satisfies ^int32[]" type=Owned<Array<int32>>
        /// @type.node source=tail type=Owned<Array<int32>>
        /// @resolution.name source=tail target=tail
        /// @resolution.place source=tail placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=tail root=tail
        /// @generic.instance source="tail satisfies ^int32[]" id=Array<int32>
        /// @generic.instance source=tail id=Array<int32>

    }
}

/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
"#,
    );
}

#[test]
fn test_match_union_pattern_requires_compatible_bindings() {
    let session = TestSession::single(
        r#"
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left: item } | { right: item } => item satisfies int32
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left: item } | { right: item } => item satisfies int32
}

=== checked ===
declare const value: { left: int32 } | { right: int32 };
/// @type.symbol symbol=value source=value type={ left: int32 } | { right: int32 }
/// @resolution.pattern source=value kind=binding target=value

match (value) {
/// @type.node type=int32
/// @type.node source=value type={ left: int32 } | { right: int32 }
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value

    { left: item } | { right: item } => item satisfies int32
    /// @resolution.pattern source={ left: item } kind=object fields={ left: item }
    /// @resolution.pattern source={ left: item } | { right: item } kind=union patterns=[pattern, pattern]
    /// @type.symbol symbol=item source=item type=int32
    /// @resolution.pattern source=item kind=binding target=item
    /// @resolution.pattern source={ right: item } kind=object fields={ right: item }
    /// @resolution.pattern source=item kind=binding target=item
    /// @type.node source="item satisfies int32" type=int32
    /// @type.node source=item type=int32
    /// @resolution.name source=item target=item
    /// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=item root=item

}
"#,
    );
}

#[test]
fn test_match_wildcard_fallback_uses_remaining_branch_type() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
    _ => status
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "error" = match (status) {
    "ready" => "go"
    _ => status
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"
/// @resolution.pattern source=status kind=binding target=status

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "error"
/// @resolution.pattern source=label kind=binding target=label
/// @type.node type="go" | "error"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status
/// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=status root=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

    _ => status
    /// @resolution.pattern source=_ kind=wildcard
    /// @type.node source=status type="error"
    /// @resolution.name source=status target=status
    /// @resolution.place source=status placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=status root=status

};
"#,
    );
}
