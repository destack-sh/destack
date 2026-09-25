use crate::tests::{DirRows, TestSession};

/// Keep static parameters independent from a shared declaration receiver.
#[test]
fn test_keep_static_parameters_independent_of_a_shared_receiver() {
    let session = TestSession::single(
        r#"
shared class Pack<out T> {}

export extension<T> of Pack<T> {
    static from(values: Iterable<T>): ^Pack<T> {
        todo("Pack.from")
    }
}

export extension<T> of ^Pack<T> implements From<Iterable<T>> {
    static from(values: Iterable<T>): ^Pack<T> {
        Pack.from(values)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
shared class Pack<out T> {}

export extension<T> of Pack<T> {
    static from(values: Iterable<T>): ^Pack<T> {
        todo("Pack.from" as string | undefined)
    }
}

export extension<T> of ^Pack<T> implements From<Iterable<T>> {
    static from(values: Iterable<T>): ^Pack<T> {
        Pack.from<T>(values)
    }
}

=== dir ===
shared class Pack<out T> {}
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack source="shared class Pack<out T> {}" type=typeof Pack
/// @definition.class symbol=Pack source="shared class Pack<out T> {}" template=(out T#1)
/// @type.symbol symbol=Pack.T source="out T" type=T#1

export extension<T> of Pack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#2>
/// @definition.method symbol=from#1 slot=from static=true type=(Iterable<T#2>) => ^Pack<T#2>
/// @type.symbol symbol=T#1 source=T type=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#1

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#1 type=(Iterable<T#2>) => ^Pack<T#2>
    /// @type.symbol symbol=from.values#1 source="values: Iterable<T>" type=Iterable<T#2>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#1

        todo("Pack.from")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Pack.from\")" parameters=(string | undefined) arguments=(provided("Pack.from") as string | undefined) return=never kind=symbol target=todo

    }
}

export extension<T> of ^Pack<T> implements From<Iterable<T>> {
/// @generic.template symbol=<module>#3 parameters=(T#3)
/// @definition.extension symbol=<module>#3 form=exported target=^Pack<T#3>
/// @definition.implements symbol=<module>#3 source=From<Iterable<T>> target=From<Iterable<T#3>>
/// @definition.method symbol=from#2 slot=from static=true type=(Iterable<T#3>) => ^Pack<T#3>
/// @definition.conformance symbol=<module>#3 member=from#2 requirement=From.from
/// @type.symbol symbol=T#2 source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#2
/// @resolution.name source=From target=From
/// @resolution.name source=Iterable target=Iterable
/// @resolution.name source=T target=T#2

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#2 type=(Iterable<T#3>) => ^Pack<T#3>
    /// @type.symbol symbol=from.values#2 source="values: Iterable<T>" type=Iterable<T#3>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#2

        Pack.from(values)
        /// @resolution.name source=Pack target=Pack
        /// @resolution.member source=Pack.from receiver=typeof Pack type=(Iterable<T#2>) => ^Pack<T#2> kind=symbol target_receiver=typeof Pack target=from#1
        /// @resolution.call source=Pack.from(values) parameters=(Iterable<T#3>) arguments=(provided(values) as Iterable<T#3>) return=^Pack<T#3> kind=symbol target=from#1 instance=Pack<T#3>.<extension#1>.from#1
        /// @generic.instantiation id=from#1<T#3> template=from#1 arguments=(T#3) owner=from#2
        /// @resolution.name source=values target=from.values#2
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=from.values#2

    }
}
"#, r#"
"#);
}

/// Place a static `this` result according to its shared declaration.
#[test]
fn test_static_this_uses_shared_declaration_placement() {
    let session = TestSession::single(
        r#"
shared class Channel {}

export extension of Channel {
    static new(): this {
        todo("Channel.new")
    }
}

const channel: Channel = Channel.new();
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
shared class Channel {}

export extension of Channel {
    static new(): Channel {
        todo("Channel.new" as string | undefined)
    }
}

const channel: Channel = Channel.new();

=== dir ===
shared class Channel {}
/// @type.symbol symbol=Channel source="shared class Channel {}" type=typeof Channel
/// @definition.class symbol=Channel source="shared class Channel {}"

export extension of Channel {
/// @definition.extension symbol=<module>#2 form=exported target=Channel
/// @definition.method symbol=new slot=new static=true type=() => Channel
/// @resolution.name source=Channel target=Channel

    static new(): this {
    /// @type.symbol symbol=new type=() => Channel

        todo("Channel.new")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Channel.new\")" parameters=(string | undefined) arguments=(provided("Channel.new") as string | undefined) return=never kind=symbol target=todo

    }
}

const channel: Channel = Channel.new();
/// @type.symbol symbol=channel source=channel type=Channel
/// @resolution.pattern source=channel kind=binding target=channel
/// @resolution.name source=Channel target=Channel
/// @resolution.name source=Channel target=Channel
/// @resolution.member source=Channel.new receiver=typeof Channel type=() => Channel kind=symbol target_receiver=typeof Channel target=new
/// @resolution.call source=Channel.new() parameters=() return=Channel kind=symbol target=new
"#, r#"
"#);
}

/// Apply inferred extension arguments to a static `this` result.
#[test]
fn test_static_this_uses_inferred_extension_arguments() {
    let session = TestSession::single(
        r#"
shared class Pack<out T> {}

export extension<T> of Pack<T> {
    static from(value: T): this {
        todo("Pack.from")
    }
}

declare const value: int32;
const pack: Pack<int32> = Pack.from(value);
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
shared class Pack<out T> {}

export extension<T> of Pack<T> {
    static from(value: T): Pack<T> {
        todo("Pack.from" as string | undefined)
    }
}

declare const value: int32;
const pack: Pack<int32> = Pack.from<int32>(value);

=== dir ===
shared class Pack<out T> {}
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack source="shared class Pack<out T> {}" type=typeof Pack
/// @definition.class symbol=Pack source="shared class Pack<out T> {}" template=(out T#1)
/// @type.symbol symbol=Pack.T source="out T" type=T#1

export extension<T> of Pack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#2>
/// @definition.method symbol=from slot=from static=true type=(T#2) => Pack<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T

    static from(value: T): this {
    /// @type.symbol symbol=from type=(T#2) => Pack<T#2>
    /// @type.symbol symbol=from.value source="value: T" type=T#2
    /// @resolution.name source=T target=T

        todo("Pack.from")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Pack.from\")" parameters=(string | undefined) arguments=(provided("Pack.from") as string | undefined) return=never kind=symbol target=todo

    }
}

declare const value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

const pack: Pack<int32> = Pack.from(value);
/// @type.symbol symbol=pack source=pack type=Pack<int32>
/// @resolution.pattern source=pack kind=binding target=pack
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=Pack target=Pack
/// @resolution.member source=Pack.from receiver=typeof Pack type=(T#2) => Pack<T#2> kind=symbol target_receiver=typeof Pack target=from
/// @resolution.call source=Pack.from(value) parameters=(int32) arguments=(provided(value) as int32) return=Pack<int32> kind=symbol target=from instance=Pack<int32>.<extension#1>.from
/// @generic.instantiation id=from<int32> template=from arguments=(int32)
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
"#, r#"
"#);
}

/// Preserve explicitly shared static parameters.
#[test]
fn test_static_shared_parameter_requires_shared_argument() {
    let session = TestSession::single(
        r#"
struct Message {}
shared class Channel {}

export extension of Channel {
    static send(message: &readonly Message): void { /* intentionally empty */ }
}

function relay(): void {
    const message: Message = Message {};
    Channel.send(message);
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
struct Message {}
shared class Channel {}

export extension of Channel {
    static send(message: &'a readonly Message): void {}
}

function relay(): void {
    const message: Message = Message {};
    Channel.send<"frame">(message as &'frame readonly Message);
}

=== dir ===
struct Message {}
/// @type.symbol symbol=Message source="struct Message {}" type=Message
/// @definition.struct symbol=Message source="struct Message {}"

shared class Channel {}
/// @type.symbol symbol=Channel source="shared class Channel {}" type=typeof Channel
/// @definition.class symbol=Channel source="shared class Channel {}"

export extension of Channel {
/// @definition.extension symbol=<module>#2 form=exported target=Channel
/// @definition.method symbol=send source="static send(message: &readonly Message): void { /* intentionally empty */ }" slot=send static=true type=<send.'a>(&send.'a readonly Message) => void
/// @resolution.name source=Channel target=Channel

    static send(message: &readonly Message): void { /* intentionally empty */ }
    /// @generic.template symbol=send parameters=('a)
    /// @type.symbol symbol=send source="static send(message: &readonly Message): void { /* intentionally empty */ }" type=<send.'a>(&send.'a readonly Message) => void
    /// @type.symbol symbol=send.message source="message: &readonly Message" type=&send.'a readonly Message
    /// @resolution.name source=Message target=Message

}

function relay(): void {
/// @type.symbol symbol=relay type=() => void

    const message: Message = Message {};
    /// @type.symbol symbol=relay.message source=message type=Message
    /// @resolution.pattern source=message kind=binding target=relay.message
    /// @resolution.name source=Message target=Message
    /// @resolution.name source=Message target=Message

    Channel.send(message);
    /// @resolution.name source=Channel target=Channel
    /// @resolution.member source=Channel.send receiver=typeof Channel type=<send.'a>(&send.'a readonly Message) => void kind=symbol target_receiver=typeof Channel target=send
    /// @resolution.call source=Channel.send(message) parameters=(&'frame readonly Message) arguments=(provided(message) as &'frame readonly Message) return=void regions=("frame" & "local") kind=symbol target=send instance="Channel.<extension#1>.send<\"frame\" & \"local\">"
    /// @generic.instantiation id="send<\"frame\" & \"local\">" template=send arguments=("frame" & "local")
    /// @resolution.name source=message target=relay.message
    /// @resolution.place source=message placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=message root=relay.message

}
"#,
        r#"

"#,
    );
}

/// Keep instance parameters relative to a shared value receiver.
#[test]
fn test_instance_shared_receiver_places_parameters() {
    let session = TestSession::single(
        r#"
struct Message {}
shared class Channel {}

export extension of Channel {
    send(&this, message: &readonly Message): void { /* intentionally empty */ }
}

declare const channel: Channel;
function relay(): void {
    const message: Message = Message {};
    channel.send(message);
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Message {}
shared class Channel {}

export extension of Channel {
    send(&this, message: &'b readonly Message): void {}
}

declare const channel: Channel;
function relay(): void {
    const message: Message = Message {};
    channel.send<"managed", "frame">(message as &'frame readonly Message);
}

=== dir ===
struct Message {}
/// @type.symbol symbol=Message source="struct Message {}" type=Message
/// @definition.struct symbol=Message source="struct Message {}"

shared class Channel {}
/// @type.symbol symbol=Channel source="shared class Channel {}" type=typeof Channel
/// @definition.class symbol=Channel source="shared class Channel {}"

export extension of Channel {
/// @definition.extension symbol=<module>#2 form=exported target=Channel
/// @definition.method symbol=send source="send(&this, message: &readonly Message): void { /* intentionally empty */ }" slot=send type=<send.'a, send.'b>(this: &send.'a Channel, &send.'b readonly Message) => void
/// @resolution.name source=Channel target=Channel

    send(&this, message: &readonly Message): void { /* intentionally empty */ }
    /// @generic.template symbol=send parameters=('a, 'b)
    /// @type.symbol symbol=send source="send(&this, message: &readonly Message): void { /* intentionally empty */ }" type=<send.'a, send.'b>(this: &send.'a Channel, &send.'b readonly Message) => void
    /// @type.symbol symbol=send.this source=&this type=&send.'a Channel
    /// @type.symbol symbol=send.message source="message: &readonly Message" type=&send.'b readonly Message
    /// @resolution.name source=Message target=Message

}

declare const channel: Channel;
/// @type.symbol symbol=channel source=channel type=Channel
/// @resolution.pattern source=channel kind=binding target=channel
/// @resolution.name source=Channel target=Channel

function relay(): void {
/// @type.symbol symbol=relay type=() => void

    const message: Message = Message {};
    /// @type.symbol symbol=relay.message source=message type=Message
    /// @resolution.pattern source=message kind=binding target=relay.message
    /// @resolution.name source=Message target=Message
    /// @resolution.name source=Message target=Message

    channel.send(message);
    /// @resolution.name source=channel target=channel
    /// @resolution.member source=channel.send receiver=Channel type=<send.'a, send.'b>(this: &send.'a Channel, &send.'b readonly Message) => void kind=symbol target_receiver=Channel target=send
    /// @resolution.call source=channel.send(message) parameters=(&'frame readonly Message) arguments=(provided(message) as &'frame readonly Message) return=void regions=("managed" & "shared", "frame" & "local") kind=symbol target=send receiver=Channel adjustments=(borrow(&'managed Channel)) instance="Channel.<extension#1>.send<\"managed\" & \"shared\", \"frame\" & \"local\">"
    /// @resolution.place source=channel placement="shared" lifetime="static" access="immutable"
    /// @resolution.access source=channel root=channel
    /// @generic.instantiation id="send<\"managed\" & \"shared\", \"frame\" & \"local\">" template=send arguments=("managed" & "shared", "frame" & "local")
    /// @resolution.name source=message target=relay.message
    /// @resolution.place source=message placement="local" lifetime="frame" access="immutable"
    /// @resolution.access source=message root=relay.message

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_duplicate_static_between_value_and_owned_form_extensions() {
    let session = TestSession::single(
        r#"
struct Pack<T> {
    value: T;
}

export extension<T: Compare<T>> of Pack<T> {
    static from(values: Iterable<T>): ^Pack<T> {
        todo("Pack.from")
    }
}

export extension<T: Compare<T>> of ^Pack<T> {
    static from(values: Iterable<T>): ^Pack<T> {
        Pack.from(values)
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

export extension<T: Compare<T>> of Pack<T> {
    static from(values: Iterable<T>): Pack<T> {
        todo("Pack.from" as string | undefined)
    }
}

export extension<T: Compare<T>> of ^Pack<T> {
    static from(values: Iterable<T>): Pack<T> {
        Pack.from<T>(values)
    }
}

=== dir ===
struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#1)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Pack.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#1
    /// @resolution.name source=T target=Pack.T

}

export extension<T: Compare<T>> of Pack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Compare<T#2>)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#2>
/// @definition.method symbol=from#1 slot=from static=true type=(Iterable<T#2>) => Pack<T#2>
/// @type.symbol symbol=T#1 source="T: Compare<T>" type=T#2
/// @resolution.name source=Compare target=Compare
/// @resolution.name source=T target=T#1
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#1

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#1 type=(Iterable<T#2>) => Pack<T#2>
    /// @type.symbol symbol=from.values#1 source="values: Iterable<T>" type=Iterable<T#2>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#1

        todo("Pack.from")
        /// @type.node source="todo(\"Pack.from\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Pack.from\")" parameters=(string | undefined) arguments=(provided("Pack.from") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"Pack.from\"" type="Pack.from"

    }
}

export extension<T: Compare<T>> of ^Pack<T> {
/// @generic.template symbol=<module>#3 parameters=(T#3: Compare<T#3>)
/// @definition.extension symbol=<module>#3 form=exported target=Pack<T#3>
/// @definition.method symbol=from#2 slot=from static=true type=(Iterable<T#3>) => Pack<T#3>
/// @type.symbol symbol=T#2 source="T: Compare<T>" type=T#3
/// @resolution.name source=Compare target=Compare
/// @resolution.name source=T target=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#2

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#2 type=(Iterable<T#3>) => Pack<T#3>
    /// @type.symbol symbol=from.values#2 source="values: Iterable<T>" type=Iterable<T#3>
    /// @resolution.name source=Iterable target=Iterable
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#2

        Pack.from(values)
        /// @type.node source=Pack type=Pack
        /// @type.node source=Pack.from type=(Iterable<T#2>) => Pack<T#2>
        /// @type.node source=Pack.from(values) type=Pack<T#3>
        /// @resolution.name source=Pack target=Pack
        /// @resolution.member source=Pack.from receiver=Pack type=(Iterable<T#2>) => Pack<T#2> kind=symbol target_receiver=Pack target=from#1
        /// @resolution.call source=Pack.from(values) parameters=(Iterable<T#3>) arguments=(provided(values) as Iterable<T#3>) return=Pack<T#3> kind=symbol target=from#1 instance=Pack<T#3>.<extension#1>.from#1
        /// @generic.instantiation id=from#1<T#3> template=from#1 arguments=(T#3) owner=from#2
        /// @type.node source=values type=Iterable<T#3>
        /// @resolution.name source=values target=from.values#2
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=from.values#2

    }
}
"#,
        r#"
/// @diagnostic.error id=ambiguous-member message="member 'from' is ambiguous"
/// @diagnostic.label line=14 column=14 span="from" line_source="Pack.from(values)"
/// @diagnostic.error id=duplicate-member message="member 'from' is already declared for 'Pack<T>' by another visible extension"
/// @diagnostic.label line=13 column=12 span="from" line_source="static from(values: Iterable<T>): ^Pack<T> {"
"#,
    );
}

#[test]
fn test_accept_an_enum_member_as_a_const_type_argument() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read,
    Write,
}

newtype Port<const out M: Mode = Mode.Read> = int32;

export type ReadPort = Port<Mode.Write>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Mode {
    Read,
    Write,
}

newtype Port<const out M: Mode = Mode.Read> = int32;

export type ReadPort = Port<Mode.Write>;

=== dir ===
enum Mode {
/// @type.symbol symbol=Mode type=Mode
/// @definition.enum symbol=Mode
/// @definition.variant symbol=Mode.Read source=Read key=Read value=0
/// @definition.variant symbol=Mode.Write source=Write key=Write value=1

    Read,
    /// @type.symbol symbol=Mode.Read source=Read type=Mode.Read

    Write,
    /// @type.symbol symbol=Mode.Write source=Write type=Mode.Write

}

newtype Port<const out M: Mode = Mode.Read> = int32;
/// @generic.template symbol=Port parameters=(const out M: Mode = Mode.Read)
/// @type.symbol symbol=Port source="newtype Port<const out M: Mode = Mode.Read> = int32" type=Port
/// @definition.newtype symbol=Port source="newtype Port<const out M: Mode = Mode.Read> = int32" template=(const out M: Mode = Mode.Read) backing=int32 constructors=[<const out M: Mode = Mode.Read>(int32) => Port<M>]
/// @type.symbol symbol=Port.M source="const out M: Mode = Mode.Read" type=M
/// @resolution.name source=Mode target=Mode
/// @resolution.name source=Mode.Read target=Mode
/// @resolution.path source=Mode.Read index=1 target=Mode.Read

export type ReadPort = Port<Mode.Write>;
/// @type.symbol symbol=ReadPort source="export type ReadPort = Port<Mode.Write>" type=Port<Mode.Write>
/// @definition.type symbol=ReadPort source="export type ReadPort = Port<Mode.Write>" value=Port<Mode.Write>
/// @resolution.name source=Port target=Port
/// @resolution.name source=Mode.Write target=Mode
/// @resolution.path source=Mode.Write index=1 target=Mode.Write
"#,
        r#"
"#,
    );
}

#[test]
fn test_sibling_static_calls_instantiate_freshly() {
    let session = TestSession::single(
        r#"
struct Ok<T> {
    value: T;
}

struct Err<E> {
    error: E;
}

newtype Outcome<T, E> = Ok<T> | Err<E>;

export extension<T, E> of Outcome<T, E> {
    static ok(value: T): Outcome<T, E> {
        Outcome(Ok { value })
    }

    static err(error: E): Outcome<T, E> {
        Outcome(Err { error })
    }

    map<U>(f: (value: T) => U): Outcome<U, E> {
        match (this) {
            Ok { value } => Outcome.ok(f(value))
            Err { error } => Outcome.err(error)
        }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Ok<out T> {
    value: T;
}

struct Err<out E> {
    error: E;
}

newtype Outcome<out T, out E> = Ok<T> | Err<E>;

export extension<T, E> of Outcome<T, E> {
    static ok(value: T): Outcome<T, E> {
        Outcome(Ok<T> { value })
    }

    static err(error: E): Outcome<T, E> {
        Outcome(Err<E> { error })
    }

    map<U>(f: (value: T) => U): Outcome<U, E> {
        match (this) {
            Ok { value } => Outcome.ok<U, E>(f(value))
            Err { error } => Outcome.err<U, E>(error)
        }
    }
}

=== dir ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(out T#2, out E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) backing=Ok<T#2> | Err<E#2> constructors=[<T#2, E#2>(Ok<T#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Err<E#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Ok<T#2> | Err<E#2>) => Outcome<T#2, E#2>]
/// @type.symbol symbol=Outcome.T source=T type=T#2
/// @type.symbol symbol=Outcome.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Outcome.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Outcome.E

export extension<T, E> of Outcome<T, E> {
/// @generic.template symbol=<module>#2 parameters=(T#3, E#3)
/// @definition.extension symbol=<module>#2 form=exported target=Outcome<T#3, E#3>
/// @definition.method symbol=err slot=err static=true type=(E#3) => Outcome<T#3, E#3>
/// @definition.method symbol=map slot=map type=<U>(this: Outcome<T#3, E#3>, (T#3) => U) => Outcome<U, E#3>
/// @definition.method symbol=ok slot=ok static=true type=(T#3) => Outcome<T#3, E#3>
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=E source=E type=E#3
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E

    static ok(value: T): Outcome<T, E> {
    /// @type.symbol symbol=ok type=(T#3) => Outcome<T#3, E#3>
    /// @type.symbol symbol=ok.value source="value: T" type=T#3
    /// @resolution.name source=T target=T
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        Outcome(Ok { value })
        /// @type.node source="Outcome(Ok { value })" type=Outcome<T#3, E#3>
        /// @type.node source=Outcome type=Outcome
        /// @resolution.name source=Outcome target=Outcome
        /// @resolution.construct source="Outcome(Ok { value })" parameters=(Ok<T#3>) arguments=(provided(Ok { value }) as Ok<T#3>) return=Outcome<T#3, E#3> kind=newtype target=Outcome backing=Ok<T#3> instance="Outcome<T#3, E#3>"
        /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=ok
        /// @type.node source="Ok { value }" type=Ok<T#3>
        /// @resolution.name source=Ok target=Ok
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=ok.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=ok.value

    }

    static err(error: E): Outcome<T, E> {
    /// @type.symbol symbol=err type=(E#3) => Outcome<T#3, E#3>
    /// @type.symbol symbol=err.error source="error: E" type=E#3
    /// @resolution.name source=E target=E
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        Outcome(Err { error })
        /// @type.node source="Outcome(Err { error })" type=Outcome<T#3, E#3>
        /// @type.node source=Outcome type=Outcome
        /// @resolution.name source=Outcome target=Outcome
        /// @resolution.construct source="Outcome(Err { error })" parameters=(Err<E#3>) arguments=(provided(Err { error }) as Err<E#3>) return=Outcome<T#3, E#3> kind=newtype target=Outcome backing=Err<E#3> instance="Outcome<T#3, E#3>"
        /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=err
        /// @type.node source="Err { error }" type=Err<E#3>
        /// @resolution.name source=Err target=Err
        /// @type.node source=error type=E#3
        /// @resolution.name source=error target=err.error
        /// @resolution.place source=error placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=error root=err.error

    }

    map<U>(f: (value: T) => U): Outcome<U, E> {
    /// @generic.template symbol=map parent=template#3 parameters=(U)
    /// @type.symbol symbol=map type=<U>(this: Outcome<T#3, E#3>, (T#3) => U) => Outcome<U, E#3>
    /// @type.symbol symbol=map.this type=Outcome<T#3, E#3>
    /// @type.symbol symbol=map.U source=U type=U
    /// @type.symbol symbol=map.f source="f: (value: T) => U" type=(T#3) => U
    /// @type.symbol symbol=map.value#1 source="value: T" type=T#3
    /// @resolution.name source=T target=T
    /// @resolution.name source=U target=map.U
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=U target=map.U
    /// @resolution.name source=E target=E

        match (this) {
        /// @type.node type=Outcome<U, E#3>
        /// @resolution.coverage exhaustive=true disjoint=true
        /// @type.node source=this type=Outcome<T#3, E#3>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Outcome<T#3, E#3>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

            Ok { value } => Outcome.ok(f(value))
            /// @resolution.name source=Ok target=Ok
            /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Outcome, Ok<T#3> | Err<E#3>), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value }
            /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=map
            /// @generic.instantiation id=Ok<T#3> template=Ok arguments=(T#3) owner=map
            /// @type.symbol symbol=map.value#2 source=value type=T#3
            /// @type.node source=Outcome type=Outcome
            /// @type.node source=Outcome.ok type=(T#3) => Outcome<T#3, E#3>
            /// @type.node source=Outcome.ok(f(value)) type=Outcome<U, E#3>
            /// @resolution.name source=Outcome target=Outcome
            /// @resolution.member source=Outcome.ok receiver=Outcome type=(T#3) => Outcome<T#3, E#3> kind=symbol target_receiver=Outcome target=ok
            /// @resolution.call source=Outcome.ok(f(value)) parameters=(U) arguments=(provided(f(value)) as U) return=Outcome<U, E#3> kind=symbol target=ok instance="Outcome<U, E#3>.<extension#1>.ok"
            /// @generic.instantiation id="ok<U, E#3>" template=ok arguments=(U, E#3) owner=map
            /// @type.node source=f type=(T#3) => U
            /// @type.node source=f(value) type=U
            /// @resolution.name source=f target=map.f
            /// @resolution.call source=f(value) parameters=(T#3) arguments=(provided(value) as T#3) return=U kind=expression target=expression
            /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=f root=map.f
            /// @type.node source=value type=T#3
            /// @resolution.name source=value target=map.value#2
            /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
            /// @resolution.access source=value root=map.value#2

            Err { error } => Outcome.err(error)
            /// @resolution.name source=Err target=Err
            /// @resolution.pattern source="Err { error }" kind=nominal_object adjustments=(newtype.payload(Outcome, Ok<T#3> | Err<E#3>), union.payload(Ok<T#3> | Err<E#3>, Err<E#3>, Err<E#3>)) target=Err instance=Err<E#3> fields={ Err.error }
            /// @generic.instantiation id=Err<E#3> template=Err arguments=(E#3) owner=map
            /// @type.symbol symbol=map.error source=error type=E#3
            /// @type.node source=Outcome type=Outcome
            /// @type.node source=Outcome.err type=(E#3) => Outcome<T#3, E#3>
            /// @type.node source=Outcome.err(error) type=Outcome<U, E#3>
            /// @resolution.name source=Outcome target=Outcome
            /// @resolution.member source=Outcome.err receiver=Outcome type=(E#3) => Outcome<T#3, E#3> kind=symbol target_receiver=Outcome target=err
            /// @resolution.call source=Outcome.err(error) parameters=(E#3) arguments=(provided(error) as E#3) return=Outcome<U, E#3> kind=symbol target=err instance="Outcome<U, E#3>.<extension#1>.err"
            /// @generic.instantiation id="err<U, E#3>" template=err arguments=(U, E#3) owner=map
            /// @type.node source=error type=E#3
            /// @resolution.name source=error target=map.error
            /// @resolution.place source=error placement="local" lifetime="frame" access="immutable"
            /// @resolution.access source=error root=map.error

        }
    }
}
"#,
        r#"
"#,
    );
}

/// Stage and instantiate statics through a type alias that names its body's root declaration.
#[test]
fn test_call_a_static_member_through_a_type_alias_receiver() {
    let session = TestSession::single(
        r#"
struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> {
    static of(value: T): Pack<T> {
        Pack { value }
    }
}

export type Packed<T> = Pack<T>;

function wrap(): Packed<string> {
    Packed.of("text")
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

export extension<T> of Pack<T> {
    static of(value: T): Pack<T> {
        Pack<T> { value }
    }
}

export type Packed<T> = Pack<T>;

function wrap(): Packed<string> {
    Packed.of<string>("text")
}

=== dir ===
struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#1)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Pack.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#1
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#2>
/// @definition.method symbol=of slot=of static=true type=(T#2) => Pack<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T

    static of(value: T): Pack<T> {
    /// @type.symbol symbol=of type=(T#2) => Pack<T#2>
    /// @type.symbol symbol=of.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T

        Pack { value }
        /// @type.node source="Pack { value }" type=Pack<T#2>
        /// @resolution.name source=Pack target=Pack
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=of.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=of.value

    }
}

export type Packed<T> = Pack<T>;
/// @generic.template symbol=Packed parameters=(T#3)
/// @type.symbol symbol=Packed source="export type Packed<T> = Pack<T>" type=Pack<T#3>
/// @definition.type symbol=Packed source="export type Packed<T> = Pack<T>" template=(T#3) value=Pack<T#3>
/// @type.symbol symbol=Packed.T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=Packed.T

function wrap(): Packed<string> {
/// @type.symbol symbol=wrap type=() => Packed<string>
/// @resolution.name source=Packed target=Packed

    Packed.of("text")
    /// @type.node source="Packed.of(\"text\")" type=Pack<string>
    /// @type.node source=Packed type=Packed
    /// @type.node source=Packed.of type=(T#2) => Pack<T#2>
    /// @resolution.name source=Packed target=Packed
    /// @resolution.member source=Packed.of receiver=Packed type=(T#2) => Pack<T#2> kind=symbol target_receiver=Packed target=of
    /// @resolution.call source="Packed.of(\"text\")" parameters=(string) arguments=(provided("text") as string) return=Pack<string> kind=symbol target=of instance=Pack<string>.<extension#1>.of
    /// @generic.instantiation id=of<string> template=of arguments=(string)
    /// @type.node source="\"text\"" type="text"

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_field_reads_through_borrows_project_deep_readonly() {
    let session = TestSession::single(
        r#"
struct Pack<T> {
    value: T;
}

function read<T>(pack: &readonly Pack<T>): readonly T {
    pack.value
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

function read<T, 'a>(pack: &'a readonly Pack<T>): readonly T {
    pack.value
}

=== dir ===
struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#1)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Pack.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#1
    /// @resolution.name source=T target=Pack.T

}

function read<T>(pack: &readonly Pack<T>): readonly T {
/// @generic.template symbol=read parameters=(T#2, 'a)
/// @type.symbol symbol=read type=<T#2, read.'a>(&read.'a readonly Pack<T#2>) => readonly T#2
/// @type.symbol symbol=read.T source=T type=T#2
/// @type.symbol symbol=read.pack source="pack: &readonly Pack<T>" type=&read.'a readonly Pack<T#2>
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    pack.value
    /// @type.node source=pack type=&read.'a readonly Pack<T#2>
    /// @type.node source=pack.value type=readonly T#2
    /// @resolution.name source=pack target=read.pack
    /// @resolution.member source=pack.value receiver=&read.'a readonly Pack<T#2> type=readonly T#2 kind=field target_receiver=&read.'a readonly Pack<T#2> key=value target=Pack.value target_type=readonly T#2
    /// @resolution.place source=pack placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=pack root=read.pack
    /// @resolution.place source=pack.value placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=pack.value root=read.pack keys=[value]

}
"#,
        r#"
"#,
    );
}

#[test]
fn test_view_arguments_select_their_union_arm() {
    let session = TestSession::single(
        r#"
struct Pack<T> {
    value: T;
}

function same<T>(actual: readonly T | T, expected: T): void {
    todo("same")
}

function check<T>(pack: &readonly Pack<T>, expected: T): void {
    same(pack.value, expected)
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

function same<T>(actual: readonly T | T, expected: T): void {
    todo("same" as string | undefined);
}

function check<T, 'a>(pack: &'a readonly Pack<T>, expected: T): void {
    same<T>(pack.value as readonly T | T, expected);
}

=== dir ===
struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#1)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#1)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Pack.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#1
    /// @resolution.name source=T target=Pack.T

}

function same<T>(actual: readonly T | T, expected: T): void {
/// @generic.template symbol=same parameters=(T#2)
/// @type.symbol symbol=same type=<T#2>(readonly T#2 | T#2, T#2) => void
/// @type.symbol symbol=same.T source=T type=T#2
/// @type.symbol symbol=same.actual source="actual: readonly T | T" type=readonly T#2 | T#2
/// @resolution.name source=T target=same.T
/// @resolution.name source=T target=same.T
/// @type.symbol symbol=same.expected source="expected: T" type=T#2
/// @resolution.name source=T target=same.T

    todo("same")
    /// @type.node source="todo(\"same\")" type=never
    /// @type.node source=todo type=(string | undefined?) => never
    /// @resolution.name source=todo target=todo
    /// @resolution.call source="todo(\"same\")" parameters=(string | undefined) arguments=(provided("same") as string | undefined) return=never kind=symbol target=todo
    /// @type.node source="\"same\"" type="same"

}

function check<T>(pack: &readonly Pack<T>, expected: T): void {
/// @generic.template symbol=check parameters=(T#3, 'a)
/// @type.symbol symbol=check type=<T#3, check.'a>(&check.'a readonly Pack<T#3>, T#3) => void
/// @type.symbol symbol=check.T source=T type=T#3
/// @type.symbol symbol=check.pack source="pack: &readonly Pack<T>" type=&check.'a readonly Pack<T#3>
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=check.T
/// @type.symbol symbol=check.expected source="expected: T" type=T#3
/// @resolution.name source=T target=check.T

    same(pack.value, expected)
    /// @type.node source="same(pack.value, expected)" type=void
    /// @type.node source=same type=(readonly T#3 | T#3, T#3) => void
    /// @resolution.name source=same target=same
    /// @resolution.call source="same(pack.value, expected)" parameters=(readonly T#3 | T#3, T#3) arguments=(provided(pack.value) as readonly T#3 | T#3, provided(expected) as T#3) return=void kind=symbol target=same instance=same<T#3>
    /// @generic.instantiation id=same<T#3> template=same arguments=(T#3) owner=check
    /// @type.node source=pack type=&check.'a readonly Pack<T#3>
    /// @type.node source=pack.value type=readonly T#3
    /// @resolution.name source=pack target=check.pack
    /// @resolution.member source=pack.value receiver=&check.'a readonly Pack<T#3> type=readonly T#3 kind=field target_receiver=&check.'a readonly Pack<T#3> key=value target=Pack.value target_type=readonly T#3
    /// @resolution.place source=pack placement=check.'a lifetime=check.'a access="readonly"
    /// @resolution.access source=pack root=check.pack
    /// @resolution.place source=pack.value placement=check.'a lifetime=check.'a access="readonly"
    /// @resolution.access source=pack.value root=check.pack keys=[value]
    /// @type.node source=expected type=T#3
    /// @resolution.name source=expected target=check.expected
    /// @resolution.place source=expected placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=expected root=check.expected

}
"#,
        r#"

"#,
    );
}
