use crate::tests::{DirRows, TestSession};

#[test]
fn test_narrow_inline_tagged_payload_field() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event =
    | { kind: "click"; x: int32 }
    | { kind: "key"; key: string };

function readX(event: Event): int32 {
    if (event.kind === "click") {
        const kind: "click" = event.kind;

        return event.x;
    }

    return 0;
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
@derive(Tagged)
newtype Event = { kind: "click"; x: int32 } | { kind: "key"; key: string };

function readX(event: Event): int32 {
    if (event.kind === ("click" as "click" | "key")) {
        const kind: "click" = event.kind;

        return event.x;
    }

    return 0;
}

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Click type=({ x: int32 }) => Event.Click
/// @type.symbol symbol=Event.Key type=({ key: string }) => Event.Key
/// @definition.newtype symbol=Event discriminator=kind backing={ kind: "click"; x: int32 } | { kind: "key"; key: string }
/// @definition.variant symbol=Event.Click key=Click discriminant=click backing={ kind: "click"; x: int32 } argument={ x: int32 }
/// @definition.variant symbol=Event.Key key=Key discriminant=key backing={ kind: "key"; key: string } argument={ key: string }

    | { kind: "click"; x: int32 }
    | { kind: "key"; key: string };

function readX(event: Event): int32 {
/// @type.symbol symbol=readX type=(Event) => int32
/// @type.symbol symbol=readX.event source="event: Event" type=Event
/// @resolution.name source=Event target=Event

    if (event.kind === "click") {
    /// @resolution.name source=event target=readX.event
    /// @resolution.member source=event.kind receiver=Event type="click" | "key" kind=projection target="variant.tag(Event, kind, \"click\" | \"key\")"
    /// @resolution.operator source="event.kind === \"click\"" type=boolean operator="===" kind=builtin operands=[event.kind as "click" | "key" families=(string), "click" as "click" | "key" families=(string)]
    /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event root=readX.event
    /// @resolution.place source=event.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event.kind root=readX.event keys=[kind]

        const kind: "click" = event.kind;
        /// @type.symbol symbol=readX.kind source=kind type="click"
        /// @resolution.pattern source=kind kind=binding target=readX.kind
        /// @resolution.name source=event target=readX.event
        /// @resolution.member source=event.kind receiver=Event.Click type="click" kind=projection target="variant.tag(Event, kind, \"click\")"
        /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event root=readX.event
        /// @resolution.place source=event.kind placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event.kind root=readX.event keys=[kind]

        return event.x;
        /// @resolution.name source=event target=readX.event
        /// @resolution.member source=event.x receiver=Event.Click type=int32 kind=field target_receiver={ kind: "click"; x: int32 } adjustments=(variant.payload(Event.Click, { kind: "click"; x: int32 })) key=x target_type=int32
        /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event root=readX.event
        /// @resolution.place source=event.x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event.x root=readX.event keys=[x]

    }

    return 0;
}
"#);
}

#[test]
fn test_inline_tagged_variants_expose_common_payload_field() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Event =
    | { kind: "click"; value: int32 }
    | { kind: "key"; value: int32 };

function read(event: Event): int32 {
    return event.value;
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
@derive(Tagged)
newtype Event = { kind: "click"; value: int32 } | { kind: "key"; value: int32 };

function read(event: Event): int32 {
    return event.value;
}

=== checked ===
@derive(Tagged)
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Event =
/// @type.symbol symbol=Event type=Event
/// @type.symbol symbol=Event.Click type=({ value: int32 }) => Event.Click
/// @type.symbol symbol=Event.Key type=({ value: int32 }) => Event.Key
/// @definition.newtype symbol=Event discriminator=kind backing={ kind: "click"; value: int32 } | { kind: "key"; value: int32 }
/// @definition.variant symbol=Event.Click key=Click discriminant=click backing={ kind: "click"; value: int32 } argument={ value: int32 }
/// @definition.variant symbol=Event.Key key=Key discriminant=key backing={ kind: "key"; value: int32 } argument={ value: int32 }

    | { kind: "click"; value: int32 }
    | { kind: "key"; value: int32 };

function read(event: Event): int32 {
/// @type.symbol symbol=read type=(Event) => int32
/// @type.symbol symbol=read.event source="event: Event" type=Event
/// @resolution.name source=Event target=Event

    return event.value;
    /// @resolution.name source=event target=read.event
    /// @resolution.member source=event.value type=int32 kind=union arms=[receiver=Event.Click, target=field(receiver={ kind: "click"; value: int32 } adjustments=(variant.payload(Event.Click, { kind: "click"; value: int32 })), target=value, type=int32), type=int32, receiver=Event.Key, target=field(receiver={ kind: "key"; value: int32 } adjustments=(variant.payload(Event.Key, { kind: "key"; value: int32 })), target=value, type=int32), type=int32]
    /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event root=read.event
    /// @resolution.place source=event.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event.value root=read.event keys=[value]

}
"#);
}

#[test]
fn test_explicit_tagged_discriminator_narrows_parent() {
    let session = TestSession::single(
        r#"
struct Click {
    type: "click";
    category: "pointer";
    x: int32;
}

struct Key {
    type: "key";
    category: "keyboard";
    key: string;
}

@derive(Tagged({ discriminator: "type" }))
newtype Event = Click | Key;

function readX(event: Event): int32 {
    if (event.type === "click") {
        return event.x;
    }

    return 0;
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Click {
    type: "click";
    category: "pointer";
    x: int32;
}

struct Key {
    type: "key";
    category: "keyboard";
    key: string;
}

@derive(Tagged({ discriminator: "type" }))
newtype Event = Click | Key;

function readX(event: Event): int32 {
    if (event.type === ("click" as "click" | "key")) {
        return event.x;
    }

    return 0;
}

=== checked ===
struct Click {
/// @type.symbol symbol=Click type=Click
/// @definition.struct symbol=Click
/// @definition.field symbol=Click.category source="category: \"pointer\"" key=category type="pointer"
/// @definition.field symbol=Click.type source="type: \"click\"" key=type type="click"
/// @definition.field symbol=Click.x source="x: int32" key=x type=int32

    type: "click";
    /// @type.symbol symbol=Click.type source="type: \"click\"" type="click"

    category: "pointer";
    /// @type.symbol symbol=Click.category source="category: \"pointer\"" type="pointer"

    x: int32;
    /// @type.symbol symbol=Click.x source="x: int32" type=int32

}

struct Key {
/// @type.symbol symbol=Key type=Key
/// @definition.struct symbol=Key
/// @definition.field symbol=Key.category source="category: \"keyboard\"" key=category type="keyboard"
/// @definition.field symbol=Key.key source="key: string" key=key type=string
/// @definition.field symbol=Key.type source="type: \"key\"" key=type type="key"

    type: "key";
    /// @type.symbol symbol=Key.type source="type: \"key\"" type="key"

    category: "keyboard";
    /// @type.symbol symbol=Key.category source="category: \"keyboard\"" type="keyboard"

    key: string;
    /// @type.symbol symbol=Key.key source="key: string" type=string

}

@derive(Tagged({ discriminator: "type" }))
/// @resolution.name source=derive target=decorator.derive.derive
/// @resolution.name source=Tagged target=decorator.derive.Tagged
/// @resolution.construct source="Tagged({ discriminator: \"type\" })" parameters=({ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) arguments=(provided({ discriminator: "type" }) as { discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }) return=Tagged kind=newtype target=decorator.derive.Tagged backing={ discriminator?: string; case?: decorator.derive.TaggedCase; names?: decorator.derive.TaggedNames }

newtype Event = Click | Key;
/// @type.symbol symbol=Event source="newtype Event = Click | Key" type=Event
/// @type.symbol symbol=Event.Click type=({ category: "pointer"; x: int32 }) => Event.Click
/// @type.symbol symbol=Event.Key type=({ category: "keyboard"; key: string }) => Event.Key
/// @definition.newtype symbol=Event source="newtype Event = Click | Key" discriminator=type backing=Click | Key
/// @definition.variant symbol=Event.Click source="newtype Event = Click | Key" key=Click discriminant=click backing=Click argument={ category: "pointer"; x: int32 }
/// @definition.variant symbol=Event.Key source="newtype Event = Click | Key" key=Key discriminant=key backing=Key argument={ category: "keyboard"; key: string }
/// @resolution.name source=Click target=Click
/// @resolution.name source=Key target=Key

function readX(event: Event): int32 {
/// @type.symbol symbol=readX type=(Event) => int32
/// @type.symbol symbol=readX.event source="event: Event" type=Event
/// @resolution.name source=Event target=Event

    if (event.type === "click") {
    /// @resolution.name source=event target=readX.event
    /// @resolution.member source=event.type receiver=Event type="click" | "key" kind=projection target="variant.tag(Event, type, \"click\" | \"key\")"
    /// @resolution.operator source="event.type === \"click\"" type=boolean operator="===" kind=builtin operands=[event.type as "click" | "key" families=(string), "click" as "click" | "key" families=(string)]
    /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event root=readX.event
    /// @resolution.place source=event.type placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=event.type root=readX.event keys=[type]

        return event.x;
        /// @resolution.name source=event target=readX.event
        /// @resolution.member source=event.x receiver=Event.Click type=int32 kind=field target_receiver=Event.Click adjustments=(variant.payload(Event.Click, Click)) key=x target=Click.x target_type=int32
        /// @resolution.place source=event placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event root=readX.event
        /// @resolution.place source=event.x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=event.x root=readX.event keys=[x]

    }

    return 0;
}
"#);
}

#[test]
fn test_tagged_newtype_discriminant_narrows_generic_value() {
    let session = TestSession::single(
        r#"
struct Yield<T> {
    kind: "yield";
    value: T;
}

struct Return {
    kind: "return";
}

@derive(Tagged)
newtype Result<T> = Yield<T> | Return;

function read<T>(result: Result<T>): T {
    if (result.kind == "yield") {
        return result.value;
    }

    throw "not yielded";
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Yield<out T> {
    kind: "yield";
    value: T;
}

struct Return {
    kind: "return";
}

@derive(Tagged)
newtype Result<out T> = Yield<T> | Return;

function read<T>(result: Result<T>): T {
    if (result.kind == "yield") {
        return result.value;
    }

    throw "not yielded";
}

=== checked ===
struct Yield<T> {
/// @generic.template symbol=Yield parameters=(out T#1)
/// @type.symbol symbol=Yield type=Yield
/// @definition.struct symbol=Yield template=(out T#1)
/// @definition.field symbol=Yield.kind source="kind: \"yield\"" key=kind type="yield"
/// @definition.field symbol=Yield.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Yield.T source=T type=T#1

    kind: "yield";
    /// @type.symbol symbol=Yield.kind source="kind: \"yield\"" type="yield"

    value: T;
    /// @type.symbol symbol=Yield.value source="value: T" type=T#1
    /// @resolution.name source=T target=Yield.T

}

struct Return {
/// @type.symbol symbol=Return type=Return
/// @definition.struct symbol=Return
/// @definition.field symbol=Return.kind source="kind: \"return\"" key=kind type="return"

    kind: "return";
    /// @type.symbol symbol=Return.kind source="kind: \"return\"" type="return"

}

@derive(Tagged)
/// @type.node source=derive type=derive
/// @resolution.name source=derive target=decorator.derive.derive
/// @type.node source=Tagged type=Tagged
/// @resolution.name source=Tagged target=decorator.derive.Tagged

newtype Result<T> = Yield<T> | Return;
/// @generic.template symbol=Result parameters=(out T#2)
/// @type.symbol symbol=Result source="newtype Result<T> = Yield<T> | Return" type=Result
/// @type.symbol symbol=Result.Return type=Result.Return<T#2>
/// @type.symbol symbol=Result.Yield type=<T#2>({ value: T#2 }) => Result.Yield<T#2>
/// @definition.newtype symbol=Result source="newtype Result<T> = Yield<T> | Return" template=(out T#2) discriminator=kind backing=Yield<T#2> | Return
/// @definition.variant symbol=Result.Return source="newtype Result<T> = Yield<T> | Return" key=Return discriminant=return backing=Return
/// @definition.variant symbol=Result.Yield source="newtype Result<T> = Yield<T> | Return" key=Yield discriminant=yield backing=Yield<T#2> argument={ value: T#2 }
/// @type.symbol symbol=Result.T source=T type=T#2
/// @resolution.name source=Yield target=Yield
/// @resolution.name source=T target=Result.T
/// @resolution.name source=Return target=Return

function read<T>(result: Result<T>): T {
/// @generic.template symbol=read parameters=(T#3)
/// @type.symbol symbol=read type=<T#3>(Result<T#3>) => T#3
/// @type.symbol symbol=read.T source=T type=T#3
/// @type.symbol symbol=read.result source="result: Result<T>" type=Result<T#3>
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    if (result.kind == "yield") {
    /// @type.node source="result.kind == \"yield\"" type=boolean
    /// @type.node source=result type=Result<T#3>
    /// @type.node source=result.kind type="yield" | "return"
    /// @resolution.name source=result target=read.result
    /// @resolution.member source=result.kind receiver=Result<T#3> type="yield" | "return" kind=projection target="variant.tag(Result<T#3>, kind, \"yield\" | \"return\")"
    /// @resolution.operator source="result.kind == \"yield\"" type=boolean operator="==" kind=builtin operands=[result.kind as "yield" | "return" families=(string), "yield" as "yield" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=read.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=read.result keys=[kind]
    /// @generic.instance source=result id=Result<T#3>
    /// @type.node source="\"yield\"" type="yield"

        return result.value;
        /// @type.node source=result type=Result.Yield<T#3>
        /// @type.node source=result.value type=T#3
        /// @resolution.name source=result target=read.result
        /// @resolution.member source=result.value receiver=Result.Yield<T#3> type=T#3 kind=field target_receiver=Result.Yield<T#3> adjustments=(variant.payload(Result.Yield, Yield<T#3>)) key=value target=Yield.value target_type=T#3
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=read.result
        /// @resolution.place source=result.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result.value root=read.result keys=[value]
        /// @generic.instance source=result id=Result<T#3>

    }

    throw "not yielded";
    /// @type.node source="throw \"not yielded\"" type=never
    /// @type.node source="\"not yielded\"" type="not yielded"

}

/// @generic.instance id=Result<T#2> template=Result arguments=(T#2)
/// @generic.instance id=Result<T#3> template=Result arguments=(T#3)
"#,
    );
}

#[test]
fn test_imported_tagged_newtype_discriminant_narrows_generic_value() {
    let session = TestSession::builder()
        .module(
            "result.ds",
            r#"
export struct Yield<T> {
    kind: "yield";
    value: T;
}

export struct Return {
    kind: "return";
}

@derive(Tagged)
export newtype Result<T> = Yield<T> | Return;
"#,
        )
        .module(
            "main.ds",
            r#"
import { Result } from "./result.ds";

function read<T>(result: Result<T>): T {
    if (result.kind == "yield") {
        return result.value;
    }

    throw "not yielded";
}
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Result } from "./result.ds";

function read<T>(result: Result<T>): T {
    if (result.kind == "yield") {
        return result.value;
    }

    throw "not yielded";
}

=== checked ===
import { Result } from "./result.ds";

function read<T>(result: Result<T>): T {
/// @generic.template symbol=read parameters=(T)
/// @type.symbol symbol=read type=<T>(result.Result<T>) => T
/// @type.symbol symbol=read.T source=T type=T
/// @type.symbol symbol=read.result source="result: Result<T>" type=result.Result<T>
/// @resolution.name source=Result target=result.Result
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    if (result.kind == "yield") {
    /// @type.node source="result.kind == \"yield\"" type=boolean
    /// @type.node source=result type=result.Result<T>
    /// @type.node source=result.kind type="yield" | "return"
    /// @resolution.name source=result target=read.result
    /// @resolution.member source=result.kind receiver=result.Result<T> type="yield" | "return" kind=projection target="variant.tag(result.Result<T>, kind, \"yield\" | \"return\")"
    /// @resolution.operator source="result.kind == \"yield\"" type=boolean operator="==" kind=builtin operands=[result.kind as "yield" | "return" families=(string), "yield" as "yield" families=(string)]
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=read.result
    /// @resolution.place source=result.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result.kind root=read.result keys=[kind]
    /// @generic.instance source=result id=result.Result<T>
    /// @type.node source="\"yield\"" type="yield"

        return result.value;
        /// @type.node source=result type=result.Result.Yield<T>
        /// @type.node source=result.value type=T
        /// @resolution.name source=result target=read.result
        /// @resolution.member source=result.value receiver=result.Result.Yield<T> type=T kind=field target_receiver=result.Result.Yield<T> adjustments=(variant.payload(result.symbol12, result.Yield<T>)) key=value target=result.Yield.value target_type=T
        /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result root=read.result
        /// @resolution.place source=result.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=result.value root=read.result keys=[value]
        /// @generic.instance source=result id=result.Result<T>

    }

    throw "not yielded";
    /// @type.node source="throw \"not yielded\"" type=never
    /// @type.node source="\"not yielded\"" type="not yielded"

}

/// @generic.instance id=result.Result<T> template=result.Result arguments=(T)
"#,
    );
}

#[test]
fn test_getter_equality_does_not_narrow_later_read() {
    let session = TestSession::single(
        r#"
interface Source {
    get kind(): "pending" | "ready";
}

function read(source: Source): "ready" {
    if (source.kind === "ready") {
        return source.kind;
    }

    return "ready";
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Source {
    get kind(): "pending" | "ready";
}

function read(source: Dynamic<Source>): "ready" {
    if (source.kind === ("ready" as "pending" | "ready")) {
        return source.kind;
    }

    return "ready";
}

=== checked ===
interface Source {
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source
/// @definition.method symbol=Source.kind source="get kind(): \"pending\" | \"ready\"" slot=kind role=getter type=(this: this) => "pending" | "ready"

    get kind(): "pending" | "ready";
    /// @type.symbol symbol=Source.kind source="get kind(): \"pending\" | \"ready\"" type=(this: this) => "pending" | "ready"

}

function read(source: Source): "ready" {
/// @type.symbol symbol=read type=(Dynamic<Source>) => "ready"
/// @type.symbol symbol=read type=(Source) => "ready"
/// @type.symbol symbol=read.source source="source: Source" type=Dynamic<Source>
/// @resolution.name source=Source target=Source

    if (source.kind === "ready") {
    /// @resolution.name source=source target=read.source
    /// @resolution.member source=source.kind receiver=Dynamic<Source> type="pending" | "ready" kind=call target="dynamic(Dynamic<Source> as Source, Source.kind)(parameters=(), arguments=(), return=\"pending\" | \"ready\")"
    /// @resolution.operator source="source.kind === \"ready\"" type=boolean operator="===" kind=builtin operands=[source.kind as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=source root=read.source

        return source.kind;
        /// @resolution.name source=source target=read.source
        /// @resolution.member source=source.kind receiver=Dynamic<Source> type="pending" | "ready" kind=call target="dynamic(Dynamic<Source> as Source, Source.kind)(parameters=(), arguments=(), return=\"pending\" | \"ready\")"
        /// @resolution.place source=source placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=source root=read.source

    }

    return "ready";
}
"#,
        r#"
/// @diagnostic.error id=return-not-assignable message="type '\"pending\" | \"ready\"' is not assignable to the declared result type '\"ready\"'"
/// @diagnostic.label line=8 column=23 span="kind" line_source="return source.kind;"
/// @diagnostic.note message="expected '\"ready\"', found '\"pending\"'"
"#,
    );
}

#[test]
fn test_index_place_equality_narrows_later_read() {
    let session = TestSession::single(
        r#"
function read(values: ("pending" | "ready")[]): "ready" {
    if (values[0] === "ready") {
        return values[0];
    }

    return "ready";
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function read(values: ("pending" | "ready")[]): "ready" {
    if (values[0] === ("ready" as "pending" | "ready")) {
        return values[0];
    }

    return "ready";
}

=== checked ===
function read(values: ("pending" | "ready")[]): "ready" {
/// @type.symbol symbol=read type=(Array<"pending" | "ready">) => "ready"
/// @type.symbol symbol=read.values source="values: (\"pending\" | \"ready\")[]" type=Array<"pending" | "ready">

    if (values[0] === "ready") {
    /// @resolution.name source=values target=read.values
    /// @resolution.operator source="values[0] === \"ready\"" type=boolean operator="===" kind=builtin operands=[values[0] as "pending" | "ready" families=(string), "ready" as "pending" | "ready" families=(string)]
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=read.values
    /// @resolution.place source=values[0] placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values[0] root=read.values keys=[0]
    /// @resolution.subscript source=values[0] type="pending" | "ready" kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'frame \"pending\" | \"ready\", \"exclusive\">)"
    /// @generic.instance source=values[0] id="Array<\"pending\" | \"ready\">.<extension#4>.index#1<\"exclusive\">"

        return values[0];
        /// @resolution.name source=values target=read.values
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=read.values
        /// @resolution.place source=values[0] placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values[0] root=read.values keys=[0]
        /// @resolution.subscript source=values[0] type="pending" | "ready" kind=call target="collections.array.index#1(parameters=(usize), arguments=(provided(0) as usize), return=memory.type.WithAccess<&'frame \"pending\" | \"ready\", \"exclusive\">)"
        /// @generic.instance source=values[0] id="Array<\"pending\" | \"ready\">.<extension#4>.index#1<\"exclusive\">"

    }

    return "ready";
}

/// @generic.instance id="Array<\"pending\" | \"ready\">.<extension#4>.index#1<\"exclusive\">" template=collections.array.index#1 arguments=("pending" | "ready", "exclusive")
"#,
        r#"

"#,
    );
}

#[test]
fn test_member_equality_narrows_parent_union() {
    let session = TestSession::single(
        r#"
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: State): int32 {
    if (state.kind == "pending") {
        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: Dynamic<Pending> | Dynamic<Fulfilled>): int32 {
    if (state.kind == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== checked ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: int32;
    /// @type.symbol symbol=Fulfilled.value source="value: int32" type=int32

}

type State = Pending | Fulfilled;
/// @type.symbol symbol=State source="type State = Pending | Fulfilled" type=Pending | Fulfilled
/// @definition.type symbol=State source="type State = Pending | Fulfilled" value=Pending | Fulfilled
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Fulfilled target=Fulfilled

function read(state: State): int32 {
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending", receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
    /// @resolution.operator source="state.kind == \"pending\"" type=boolean operator="==" kind=builtin operands=[state.kind as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending>
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Dynamic<Pending> type=int32 kind=field target_receiver=Dynamic<Pending> dispatch=dynamic constraint=Pending key=reactions target=Pending.reactions target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.reactions placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.reactions root=read.state keys=[reactions]

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
    );
}

#[test]
fn test_static_index_equality_narrows_parent_union() {
    let session = TestSession::single(
        r#"
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: State): int32 {
    if (state["kind"] == "pending") {
        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(state: Dynamic<Pending> | Dynamic<Fulfilled>): int32 {
    if (state["kind"] == "pending") {
        return state.reactions;
    }

    return state.value;
}

=== checked ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: int32;
    /// @type.symbol symbol=Fulfilled.value source="value: int32" type=int32

}

type State = Pending | Fulfilled;
/// @type.symbol symbol=State source="type State = Pending | Fulfilled" type=Pending | Fulfilled
/// @definition.type symbol=State source="type State = Pending | Fulfilled" value=Pending | Fulfilled
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Fulfilled target=Fulfilled

function read(state: State): int32 {
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State) => int32
/// @type.symbol symbol=read.state source="state: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    if (state["kind"] == "pending") {
    /// @type.node source="state[\"kind\"] == \"pending\"" type=boolean
    /// @type.node source="state[\"kind\"]" type="pending" | "fulfilled"
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.name source=state target=read.state
    /// @resolution.operator source="state[\"kind\"] == \"pending\"" type=boolean operator="==" kind=builtin operands=[state["kind"] as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source="state[\"kind\"]" placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source="state[\"kind\"]" root=read.state keys=[kind]
    /// @resolution.subscript source="state[\"kind\"]" type="pending" | "fulfilled" kind=union arms=[member(receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending"), member(receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled")]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @type.node source="\"kind\"" type="kind"
    /// @type.node source="\"pending\"" type="pending"

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending>
        /// @type.node source=state.reactions type=int32
        /// @resolution.name source=state target=read.state
        /// @resolution.member source=state.reactions receiver=Dynamic<Pending> type=int32 kind=field target_receiver=Dynamic<Pending> dispatch=dynamic constraint=Pending key=reactions target=Pending.reactions target_type=int32
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state
        /// @resolution.place source=state.reactions placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state.reactions root=read.state keys=[reactions]

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
    );
}

#[test]
fn test_assignment_clears_member_discriminant_narrowing() {
    let session = TestSession::single(
        r#"
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(initial: State, next: State): int32 {
    let state = initial;
    if (state.kind == "pending") {
        state = next;

        return state.reactions;
    }

    return state.value;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Pending {
    kind: "pending";
    reactions: int32;
}

interface Fulfilled {
    kind: "fulfilled";
    value: int32;
}

type State = Pending | Fulfilled;

function read(
    initial: Dynamic<Pending> | Dynamic<Fulfilled>,
    next: Dynamic<Pending> | Dynamic<Fulfilled>,
): int32 {
    let state: Dynamic<Pending> | Dynamic<Fulfilled> = initial;
    if (state.kind == "pending") {
        state = next;

        return state.reactions;
    }

    return state.value;
}

=== checked ===
interface Pending {
/// @type.symbol symbol=Pending type=Pending
/// @definition.interface symbol=Pending
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.reactions source="reactions: int32" key=reactions type=int32

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    reactions: int32;
    /// @type.symbol symbol=Pending.reactions source="reactions: int32" type=int32

}

interface Fulfilled {
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.interface symbol=Fulfilled
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: int32" key=value type=int32

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: int32;
    /// @type.symbol symbol=Fulfilled.value source="value: int32" type=int32

}

type State = Pending | Fulfilled;
/// @type.symbol symbol=State source="type State = Pending | Fulfilled" type=Pending | Fulfilled
/// @definition.type symbol=State source="type State = Pending | Fulfilled" value=Pending | Fulfilled
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=Fulfilled target=Fulfilled

function read(initial: State, next: State): int32 {
/// @type.symbol symbol=read type=(Dynamic<Pending> | Dynamic<Fulfilled>, Dynamic<Pending> | Dynamic<Fulfilled>) => int32
/// @type.symbol symbol=read type=(State, State) => int32
/// @type.symbol symbol=read.initial source="initial: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State
/// @type.symbol symbol=read.next source="next: State" type=Dynamic<Pending> | Dynamic<Fulfilled>
/// @resolution.name source=State target=State

    let state = initial;
    /// @type.symbol symbol=read.state source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.pattern source=state kind=binding target=read.state
    /// @type.node source=initial type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @resolution.name source=initial target=read.initial
    /// @resolution.access source=initial root=read.initial

    if (state.kind == "pending") {
    /// @type.node source="state.kind == \"pending\"" type=boolean
    /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
    /// @type.node source=state.kind type="pending" | "fulfilled"
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Dynamic<Pending>, target=field(receiver=dynamic(Dynamic<Pending>, constraint=Pending), target=Pending.kind, type="pending"), type="pending", receiver=Dynamic<Fulfilled>, target=field(receiver=dynamic(Dynamic<Fulfilled>, constraint=Fulfilled), target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
    /// @resolution.operator source="state.kind == \"pending\"" type=boolean operator="==" kind=builtin operands=[state.kind as "pending" | "fulfilled" families=(string), "pending" as "pending" families=(string)]
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.kind placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.kind root=read.state keys=[kind]
    /// @type.node source="\"pending\"" type="pending"

        state = next;
        /// @type.node source="state = next" type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @resolution.pattern.assign source=state kind=place
        /// @resolution.access source=state root=read.state
        /// @resolution.assignment source=state write=binding(read.state) type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=next type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @resolution.name source=next target=read.next
        /// @resolution.place source=next placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=next root=read.next

        return state.reactions;
        /// @type.node source=state type=Dynamic<Pending> | Dynamic<Fulfilled>
        /// @type.node source=state.reactions type=<error>
        /// @resolution.name source=state target=read.state
        /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=state root=read.state

    }

    return state.value;
    /// @type.node source=state type=Dynamic<Fulfilled>
    /// @type.node source=state.value type=int32
    /// @resolution.name source=state target=read.state
    /// @resolution.member source=state.value receiver=Dynamic<Fulfilled> type=int32 kind=field target_receiver=Dynamic<Fulfilled> dispatch=dynamic constraint=Fulfilled key=value target=Fulfilled.value target_type=int32
    /// @resolution.place source=state placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state root=read.state
    /// @resolution.place source=state.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=state.value root=read.state keys=[value]

}
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'reactions' does not exist on type 'Dynamic<Pending> | Dynamic<Fulfilled>'"
/// @diagnostic.label line=19 column=22 span="reactions" line_source="return state.reactions;"
"#,
    );
}

#[test]
fn test_early_return_narrows_a_member_discriminant() {
    let session = TestSession::single(
        r#"
class Waiter<T> {
    value: T;
    next: Waiter<T> | undefined;

    constructor(value: T) {
        this.value = value;
        this.next = undefined;
    }
}

struct Pending<T> {
    kind: "pending";
    head: Waiter<T> | undefined;
    tail: Waiter<T> | undefined;
}

struct Fulfilled<T> {
    kind: "fulfilled";
    value: T;
}

type State<T> = Pending<T> | Fulfilled<T>;

class Cell<T> {
    state: State<T>;

    constructor(pending: Pending<T>) {
        this.state = pending;
    }

    consume(value: T): void {
        value;
    }

    poke(waiter: Waiter<T>): void {
        if (this.state.kind == "fulfilled") {
            this.consume(this.state.value);
            return;
        }

        if (this.state.tail == undefined) {
            this.state.head = waiter;
            this.state.tail = waiter;
        } else {
            this.state.tail.next = waiter;
            this.state.tail = waiter;
        }
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Waiter<in out T> {
    value: T;
    next: Waiter<T> | undefined;

    constructor(value: T): this {
        this.value = value;
        this.next = undefined as Waiter<T> | undefined;
    }
}

struct Pending<in out T> {
    kind: "pending";
    head: Waiter<T> | undefined;
    tail: Waiter<T> | undefined;
}

struct Fulfilled<out T> {
    kind: "fulfilled";
    value: T;
}

type State<T> = Pending<T> | Fulfilled<T>;

class Cell<in out T> {
    state: State<T>;

    constructor(pending: Pending<T>): this {
        this.state = pending as Pending<T> | Fulfilled<T>;
    }

    consume(value: T): void {
        value;
    }

    poke(waiter: Waiter<T>): void {
        if (this.state.kind == "fulfilled") {
            this.consume<T>(this.state.value);
            return;
        }

        if (this.state.tail == undefined) {
            this.state.head = waiter as Waiter<T> | undefined;
            this.state.tail = waiter as Waiter<T> | undefined;
        } else {
            this.state.tail.next = waiter as Waiter<T> | undefined;
            this.state.tail = waiter as Waiter<T> | undefined;
        }
    }
}

=== checked ===
class Waiter<T> {
/// @generic.template symbol=Waiter parameters=(in out T#1)
/// @type.symbol symbol=Waiter type=Waiter
/// @definition.class symbol=Waiter template=(in out T#1)
/// @definition.field symbol=Waiter.next source="next: Waiter<T> | undefined" key=next type=Waiter<T#1> | undefined
/// @definition.field symbol=Waiter.value source="value: T" key=value type=T#1
/// @definition.method symbol=Waiter.constructor slot=constructor role=constructor type=(T#1) => this
/// @type.symbol symbol=Waiter.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Waiter.value source="value: T" type=T#1
    /// @resolution.name source=T target=Waiter.T

    next: Waiter<T> | undefined;
    /// @type.symbol symbol=Waiter.next source="next: Waiter<T> | undefined" type=Waiter<T#1> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Waiter.T

    constructor(value: T) {
    /// @type.symbol symbol=Waiter.constructor type=(T#1) => this
    /// @type.symbol symbol=Waiter.constructor.value source="value: T" type=T#1
    /// @resolution.name source=T target=Waiter.T

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Waiter type=Waiter<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Waiter<T#1>, target=field(receiver=Waiter<T#1>, target=Waiter.value, type=T#1), type=T#1" type=T#1
        /// @resolution.name source=value target=Waiter.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Waiter.constructor.value

        this.next = undefined;
        /// @resolution.receiver source=this kind=this declaration=Waiter type=Waiter<T#1>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.next kind=place
        /// @resolution.access source=this.next root=this keys=[next]
        /// @resolution.assignment source=this.next write="receiver=Waiter<T#1>, target=field(receiver=Waiter<T#1>, target=Waiter.next, type=Waiter<T#1> | undefined), type=Waiter<T#1> | undefined" type=Waiter<T#1> | undefined

    }
}

struct Pending<T> {
/// @generic.template symbol=Pending parameters=(in out T#2)
/// @type.symbol symbol=Pending type=Pending
/// @definition.struct symbol=Pending template=(in out T#2)
/// @definition.field symbol=Pending.head source="head: Waiter<T> | undefined" key=head type=Waiter<T#2> | undefined
/// @definition.field symbol=Pending.kind source="kind: \"pending\"" key=kind type="pending"
/// @definition.field symbol=Pending.tail source="tail: Waiter<T> | undefined" key=tail type=Waiter<T#2> | undefined
/// @type.symbol symbol=Pending.T source=T type=T#2

    kind: "pending";
    /// @type.symbol symbol=Pending.kind source="kind: \"pending\"" type="pending"

    head: Waiter<T> | undefined;
    /// @type.symbol symbol=Pending.head source="head: Waiter<T> | undefined" type=Waiter<T#2> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Pending.T

    tail: Waiter<T> | undefined;
    /// @type.symbol symbol=Pending.tail source="tail: Waiter<T> | undefined" type=Waiter<T#2> | undefined
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Pending.T

}

struct Fulfilled<T> {
/// @generic.template symbol=Fulfilled parameters=(out T#3)
/// @type.symbol symbol=Fulfilled type=Fulfilled
/// @definition.struct symbol=Fulfilled template=(out T#3)
/// @definition.field symbol=Fulfilled.kind source="kind: \"fulfilled\"" key=kind type="fulfilled"
/// @definition.field symbol=Fulfilled.value source="value: T" key=value type=T#3
/// @type.symbol symbol=Fulfilled.T source=T type=T#3

    kind: "fulfilled";
    /// @type.symbol symbol=Fulfilled.kind source="kind: \"fulfilled\"" type="fulfilled"

    value: T;
    /// @type.symbol symbol=Fulfilled.value source="value: T" type=T#3
    /// @resolution.name source=T target=Fulfilled.T

}

type State<T> = Pending<T> | Fulfilled<T>;
/// @generic.template symbol=State parameters=(T#4)
/// @type.symbol symbol=State source="type State<T> = Pending<T> | Fulfilled<T>" type=Pending<T#4> | Fulfilled<T#4>
/// @definition.type symbol=State source="type State<T> = Pending<T> | Fulfilled<T>" template=(T#4) value=Pending<T#4> | Fulfilled<T#4>
/// @type.symbol symbol=State.T source=T type=T#4
/// @resolution.name source=Pending target=Pending
/// @resolution.name source=T target=State.T
/// @resolution.name source=Fulfilled target=Fulfilled
/// @resolution.name source=T target=State.T

class Cell<T> {
/// @generic.template symbol=Cell parameters=(in out T#5)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(in out T#5)
/// @definition.field symbol=Cell.state source="state: State<T>" key=state type=State<T#5>
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(Pending<T#5>) => this
/// @definition.method symbol=Cell.consume slot=consume type=(this: this, T#5) => void
/// @definition.method symbol=Cell.poke slot=poke type=(this: this, Waiter<T#5>) => void
/// @type.symbol symbol=Cell.T source=T type=T#5

    state: State<T>;
    /// @type.symbol symbol=Cell.state source="state: State<T>" type=State<T#5> reduced=Pending<T#5> | Fulfilled<T#5>
    /// @resolution.name source=State target=State
    /// @resolution.name source=T target=Cell.T

    constructor(pending: Pending<T>) {
    /// @type.symbol symbol=Cell.constructor type=(Pending<T#5>) => this
    /// @type.symbol symbol=Cell.constructor.pending source="pending: Pending<T>" type=Pending<T#5>
    /// @resolution.name source=Pending target=Pending
    /// @resolution.name source=T target=Cell.T

        this.state = pending;
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.state kind=place
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.assignment source=this.state write="receiver=Cell<T#5>, target=field(receiver=Cell<T#5>, target=Cell.state, type=State<T#5>), type=State<T#5>" type=State<T#5>
        /// @resolution.name source=pending target=Cell.constructor.pending
        /// @resolution.place source=pending placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=pending root=Cell.constructor.pending

    }

    consume(value: T): void {
    /// @type.symbol symbol=Cell.consume type=(this: this, T#5) => void
    /// @type.symbol symbol=Cell.consume.value source="value: T" type=T#5
    /// @resolution.name source=T target=Cell.T

        value;
        /// @resolution.name source=value target=Cell.consume.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Cell.consume.value

    }

    poke(waiter: Waiter<T>): void {
    /// @type.symbol symbol=Cell.poke type=(this: this, Waiter<T#5>) => void
    /// @type.symbol symbol=Cell.poke.waiter source="waiter: Waiter<T>" type=Waiter<T#5>
    /// @resolution.name source=Waiter target=Waiter
    /// @resolution.name source=T target=Cell.T

        if (this.state.kind == "fulfilled") {
        /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
        /// @resolution.member source=this.state.kind type="pending" | "fulfilled" kind=union arms=[receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.kind, type="pending"), type="pending", receiver=Fulfilled<T#5>, target=field(receiver=Fulfilled<T#5>, target=Fulfilled.kind, type="fulfilled"), type="fulfilled"]
        /// @resolution.operator source="this.state.kind == \"fulfilled\"" type=boolean operator="==" kind=builtin operands=[this.state.kind as "pending" | "fulfilled" families=(string), "fulfilled" as "fulfilled" families=(string)]
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.place source=this.state.kind placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state.kind root=this keys=[state, kind]

            this.consume(this.state.value);
            /// @resolution.member source=this.consume receiver=Cell<T#5> type=(this: Cell<T#5>, T#5) => void kind=symbol target_receiver=Cell<T#5> target=Cell.consume
            /// @resolution.call source=this.consume(this.state.value) parameters=(T#5) arguments=(provided(this.state.value) as T#5) return=void kind=symbol target=Cell.consume receiver=Cell<T#5> instance=Cell<T#5>.consume
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @generic.instance source=this.consume(this.state.value) id=Cell<T#5>.consume
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.member source=this.state.value receiver=Fulfilled<T#5> type=T#5 kind=field target_receiver=Fulfilled<T#5> key=value target=Fulfilled.value target_type=T#5
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.place source=this.state.value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state.value root=this keys=[state, value]

            return;
        }

        if (this.state.tail == undefined) {
        /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
        /// @resolution.member source=this.state.tail receiver=Pending<T#5> type=Waiter<T#5> | undefined kind=field target_receiver=Pending<T#5> key=tail target=Pending.tail target_type=Waiter<T#5> | undefined
        /// @resolution.operator source="this.state.tail == undefined" type=boolean operator="==" kind=builtin operands=[this.state.tail as Waiter<T#5> | undefined, undefined as undefined families=(undefined)]
        /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.place source=this.state.tail placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this.state.tail root=this keys=[state, tail]

            this.state.head = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.head kind=place
            /// @resolution.access source=this.state.head root=this keys=[state, head]
            /// @resolution.assignment source=this.state.head write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.head, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.tail kind=place
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.assignment source=this.state.tail write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.tail, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

        } else {
            this.state.tail.next = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.member source=this.state.tail receiver=Pending<T#5> type=Waiter<T#5> | undefined kind=field target_receiver=Pending<T#5> key=tail target=Pending.tail target_type=Waiter<T#5> | undefined
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.place source=this.state.tail placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.pattern.assign source=this.state.tail.next kind=place
            /// @resolution.access source=this.state.tail.next root=this keys=[state, tail, next]
            /// @resolution.assignment source=this.state.tail.next write="receiver=Waiter<T#5>, target=field(receiver=Waiter<T#5>, target=Waiter.next, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

            this.state.tail = waiter;
            /// @resolution.member source=this.state receiver=Cell<T#5> type=State<T#5> kind=field target_receiver=Cell<T#5> key=state target=Cell.state target_type=State<T#5>
            /// @resolution.receiver source=this kind=this declaration=Cell type=Cell<T#5>
            /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this root=this
            /// @resolution.place source=this.state placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=this.state root=this keys=[state]
            /// @resolution.pattern.assign source=this.state.tail kind=place
            /// @resolution.access source=this.state.tail root=this keys=[state, tail]
            /// @resolution.assignment source=this.state.tail write="receiver=Pending<T#5>, target=field(receiver=Pending<T#5>, target=Pending.tail, type=Waiter<T#5> | undefined), type=Waiter<T#5> | undefined" type=Waiter<T#5> | undefined
            /// @resolution.name source=waiter target=Cell.poke.waiter
            /// @resolution.place source=waiter placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=waiter root=Cell.poke.waiter

        }
    }
}

/// @generic.instance id=Cell<T#5>.consume template=Cell.consume arguments=(T#5)
/// @generic.instance id=Fulfilled<T#4> template=Fulfilled arguments=(T#4)
/// @generic.instance id=Pending<T#4> template=Pending arguments=(T#4)
/// @generic.instance id=Pending<T#5> template=Pending arguments=(T#5)
/// @generic.instance id=State<T#5> template=State arguments=(T#5)
/// @generic.instance id=Waiter<T#1> template=Waiter arguments=(T#1)
/// @generic.instance id=Waiter<T#2> template=Waiter arguments=(T#2)
/// @generic.instance id=Waiter<T#5> template=Waiter arguments=(T#5)
"#,
    );
}
