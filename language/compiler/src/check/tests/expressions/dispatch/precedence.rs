use crate::tests::{DirRows, TestSession};

#[test]
fn test_sibling_static_call_with_interface_params_selects_without_cycling() {
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

export extension<T: Compare<T>> of Pack<T> {
    static from(values: Dynamic<Iterable<T, void>>): ^Pack<T> {
        todo("Pack.from" as string | undefined)
    }
}

export extension<T: Compare<T>> of ^Pack<T> {
    static from(values: Dynamic<Iterable<T, void>>): ^Pack<T> {
        Pack.from<T>(values)
    }
}

=== checked ===
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
/// @definition.method symbol=from#1 slot=from static=true type=(Dynamic<Iterable<T#2, void>>) => Owned<Pack<T#2>>
/// @type.symbol symbol=T#1 source="T: Compare<T>" type=T#2
/// @resolution.name source=Compare target=ops.comparison.Compare
/// @resolution.name source=T target=T#1
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#1

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#1 type=(Dynamic<Iterable<T#2, void>>) => Owned<Pack<T#2>> reduced=(Dynamic<Iterable<T#2, void>>) => Pack<T#2>
    /// @type.symbol symbol=from.values#1 source="values: Iterable<T>" type=Dynamic<Iterable<T#2, void>>
    /// @resolution.name source=Iterable target=iter.iterator.Iterable
    /// @resolution.name source=T target=T#1
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#1

        todo("Pack.from")
        /// @type.node source="todo(\"Pack.from\")" type=never
        /// @type.node source=todo type=(string | undefined?) => never
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Pack.from\")" parameters=(string | undefined) arguments=(provided("Pack.from") as string | undefined) return=never kind=symbol target=error.panic.todo
        /// @type.node source="\"Pack.from\"" type="Pack.from"

    }
}

export extension<T: Compare<T>> of ^Pack<T> {
/// @generic.template symbol=<module>#3 parameters=(T#3: Compare<T#3>)
/// @definition.extension symbol=<module>#3 form=exported target=Owned<Pack<T#3>>
/// @definition.method symbol=from#2 slot=from static=true type=(Dynamic<Iterable<T#3, void>>) => Owned<Pack<T#3>>
/// @type.symbol symbol=T#2 source="T: Compare<T>" type=T#3
/// @resolution.name source=Compare target=ops.comparison.Compare
/// @resolution.name source=T target=T#2
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T#2

    static from(values: Iterable<T>): ^Pack<T> {
    /// @type.symbol symbol=from#2 type=(Dynamic<Iterable<T#3, void>>) => Owned<Pack<T#3>> reduced=(Dynamic<Iterable<T#3, void>>) => Pack<T#3>
    /// @type.symbol symbol=from.values#2 source="values: Iterable<T>" type=Dynamic<Iterable<T#3, void>>
    /// @resolution.name source=Iterable target=iter.iterator.Iterable
    /// @resolution.name source=T target=T#2
    /// @resolution.name source=Pack target=Pack
    /// @resolution.name source=T target=T#2

        Pack.from(values)
        /// @type.node source=Pack type=Pack
        /// @type.node source=Pack.from type=(Dynamic<Iterable<T#2, void>>) => Owned<Pack<T#2>> & (Dynamic<Iterable<T#3, void>>) => Owned<Pack<T#3>> reduced=(Dynamic<Iterable<T#2, void>>) => Pack<T#2> & (Dynamic<Iterable<T#3, void>>) => Pack<T#3>
        /// @type.node source=Pack.from(values) type=Owned<Pack<T#3>> reduced=Pack<T#3>
        /// @resolution.name source=Pack target=Pack
        /// @resolution.member source=Pack.from receiver=Pack type=(Dynamic<Iterable<T#2, void>>) => Owned<Pack<T#2>> & (Dynamic<Iterable<T#3, void>>) => Owned<Pack<T#3>> kind=existential targets=[from#1, from#2]
        /// @resolution.call source=Pack.from(values) parameters=(Dynamic<Iterable<T#3, void>>) arguments=(provided(values) as Dynamic<Iterable<T#3, void>>) return=Owned<Pack<T#3>> kind=symbol target=from#1 receiver=Pack instance=Pack<T#3>.<extension#1>.from#1
        /// @generic.instance source=Pack.from id="Iterable<T#2, void>"
        /// @generic.instance source=Pack.from id="Iterable<T#3, void>"
        /// @generic.instance source=Pack.from id=Pack<T#2>
        /// @generic.instance source=Pack.from id=Pack<T#3>
        /// @generic.instance source=Pack.from(values) id=Pack<T#3>
        /// @generic.instance source=Pack.from(values) id=Pack<T#3>.<extension#1>.from#1
        /// @type.node source=values type=Dynamic<Iterable<T#3, void>>
        /// @resolution.name source=values target=from.values#2
        /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=values root=from.values#2
        /// @generic.instance source=values id="Iterable<T#3, void>"

    }
}

/// @generic.instance id="Iterable<T#2, void>" template=iter.iterator.Iterable arguments=(T#2, void)
/// @generic.instance id="Iterable<T#3, void>" template=iter.iterator.Iterable arguments=(T#3, void)
/// @generic.instance id=Pack<T#2> template=Pack arguments=(T#2)
/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)
/// @generic.instance id=Pack<T#3>.<extension#1>.from#1 template=from#1 arguments=(T#3)
"#,
        r#""#,
    );
}

#[test]
fn test_comptime_enum_arguments_prove_by_member_value() {
    let session = TestSession::single(
        r#"
enum Mode {
    Read,
    Write,
}

newtype Port<comptime M: Mode = Mode.Read> = int32;

export type ReadPort = Port<Mode.Write>;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
enum Mode {
    Read,
    Write,
}

newtype Port<comptime M: Mode = Mode.Read> = int32;

export type ReadPort = Port<Mode.Write>;

=== checked ===
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

newtype Port<comptime M: Mode = Mode.Read> = int32;
/// @generic.template symbol=Port parameters=(comptime M: Mode = Mode.Read)
/// @type.symbol symbol=Port source="newtype Port<comptime M: Mode = Mode.Read> = int32" type=Port
/// @definition.newtype symbol=Port source="newtype Port<comptime M: Mode = Mode.Read> = int32" template=(comptime M: Mode = Mode.Read) backing=int32
/// @type.symbol symbol=Port.M source="comptime M: Mode = Mode.Read" type=M
/// @resolution.name source=Mode target=Mode
/// @type.node source=Mode type=Mode
/// @type.node source=Mode.Read type=Mode.Read reduced=0
/// @resolution.name source=Mode target=Mode

export type ReadPort = Port<Mode.Write>;
/// @type.symbol symbol=ReadPort source="export type ReadPort = Port<Mode.Write>" type=Port<Mode.Write> reduced=Port<1>
/// @definition.type symbol=ReadPort source="export type ReadPort = Port<Mode.Write>" value=Port<Mode.Write> reduced=Port<1>
/// @resolution.name source=Port target=Port
/// @resolution.name source=Mode.Write target=Mode

/// @generic.instance id=Port<Mode.Write> template=Port arguments=(Mode.Write)
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
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

    map<U>(f: (arg0: T) => U): Outcome<U, E> {
        match (this) {
            Ok { value } => Outcome.ok<U, E>(f(value))
            Err { error } => Outcome.err<U, E>(error)
        }
    }
}

=== checked ===
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
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) backing=Ok<T#2> | Err<E#2>
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
/// @definition.method symbol=map slot=map type=<U>(this: this, Function<(T#3,), U>) => Outcome<U, E#3>
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
        /// @generic.instance source="Outcome(Ok { value })" id="Outcome<T#3, E#3>"
        /// @type.node source="Ok { value }" type=Ok<T#3>
        /// @resolution.name source=Ok target=Ok
        /// @generic.instance source="Ok { value }" id=Ok<T#3>
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
        /// @generic.instance source="Outcome(Err { error })" id="Outcome<T#3, E#3>"
        /// @type.node source="Err { error }" type=Err<E#3>
        /// @resolution.name source=Err target=Err
        /// @generic.instance source="Err { error }" id=Err<E#3>
        /// @type.node source=error type=E#3
        /// @resolution.name source=error target=err.error
        /// @resolution.place source=error placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=error root=err.error

    }

    map<U>(f: (value: T) => U): Outcome<U, E> {
    /// @generic.template symbol=map parent=template#3 parameters=(U)
    /// @type.symbol symbol=map type=<U>(this: this, Function<(T#3,), U>) => Outcome<U, E#3>
    /// @type.symbol symbol=map.U source=U type=U
    /// @type.symbol symbol=map.f source="f: (value: T) => U" type=Function<(T#3,), U>
    /// @resolution.name source=T target=T
    /// @resolution.name source=U target=map.U
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=U target=map.U
    /// @resolution.name source=E target=E

        match (this) {
        /// @type.node type=Outcome<U, E#3>
        /// @type.node source=this type=Outcome<T#3, E#3>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Outcome<T#3, E#3>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instance source=this id="Outcome<T#3, E#3>"

            Ok { value } => Outcome.ok(f(value))
            /// @resolution.name source=Ok target=Ok
            /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<T#3> fields={ Ok.value }
            /// @generic.instance source="Ok { value }" id=Ok<T#3>
            /// @type.symbol symbol=map.value#2 source=value type=T#3
            /// @type.node source=Outcome type=Outcome
            /// @type.node source=Outcome.ok type=(T#3) => Outcome<T#3, E#3>
            /// @type.node source=Outcome.ok(f(value)) type=Outcome<U, E#3>
            /// @resolution.name source=Outcome target=Outcome
            /// @resolution.member source=Outcome.ok receiver=Outcome type=(T#3) => Outcome<T#3, E#3> kind=symbol target_receiver=Outcome target=ok
            /// @resolution.call source=Outcome.ok(f(value)) parameters=(U) arguments=(provided(f(value)) as U) return=Outcome<U, E#3> kind=symbol target=ok receiver=Outcome instance="Outcome<U, E#3>.<extension#1>.ok"
            /// @generic.instance source=Outcome.ok id="Outcome<T#3, E#3>"
            /// @generic.instance source=Outcome.ok(f(value)) id="Outcome<U, E#3>"
            /// @generic.instance source=Outcome.ok(f(value)) id="Outcome<U, E#3>.<extension#1>.ok"
            /// @type.node source=f type=Function<(T#3,), U>
            /// @type.node source=f(value) type=U
            /// @resolution.name source=f target=map.f
            /// @resolution.call source=f(value) parameters=(T#3) arguments=(provided(value) as T#3) return=U kind=expression target=expression
            /// @resolution.place source=f placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=f root=map.f
            /// @type.node source=value type=T#3
            /// @resolution.name source=value target=map.value#2
            /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=value root=map.value#2

            Err { error } => Outcome.err(error)
            /// @resolution.name source=Err target=Err
            /// @resolution.pattern source="Err { error }" kind=nominal_object target=Err instance=Err<E#3> fields={ Err.error }
            /// @generic.instance source="Err { error }" id=Err<E#3>
            /// @type.symbol symbol=map.error source=error type=E#3
            /// @type.node source=Outcome type=Outcome
            /// @type.node source=Outcome.err type=(E#3) => Outcome<T#3, E#3>
            /// @type.node source=Outcome.err(error) type=Outcome<U, E#3>
            /// @resolution.name source=Outcome target=Outcome
            /// @resolution.member source=Outcome.err receiver=Outcome type=(E#3) => Outcome<T#3, E#3> kind=symbol target_receiver=Outcome target=err
            /// @resolution.call source=Outcome.err(error) parameters=(E#3) arguments=(provided(error) as E#3) return=Outcome<U, E#3> kind=symbol target=err receiver=Outcome instance="Outcome<U, E#3>.<extension#1>.err"
            /// @generic.instance source=Outcome.err id="Outcome<T#3, E#3>"
            /// @generic.instance source=Outcome.err(error) id="Outcome<U, E#3>"
            /// @generic.instance source=Outcome.err(error) id="Outcome<U, E#3>.<extension#1>.err"
            /// @type.node source=error type=E#3
            /// @resolution.name source=error target=map.error
            /// @resolution.place source=error placement="local" lifetime="frame" access="exclusive"
            /// @resolution.access source=error root=map.error

        }
    }
}

/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id="Outcome<U, E#3>" template=Outcome arguments=(U, E#3)
/// @generic.instance id="Outcome<U, E#3>.<extension#1>.err" template=err arguments=(U, E#3)
/// @generic.instance id="Outcome<U, E#3>.<extension#1>.ok" template=ok arguments=(U, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
"#,
        r#""#,
    );
}

#[test]
fn test_alias_named_receivers_reach_the_root_statics() {
    // a type alias in static-receiver position names its body's root
    // declaration, so its statics resolve and instantiate freshly
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
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

=== checked ===
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
        /// @generic.instance source="Pack { value }" id=Pack<T#2>
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
    /// @resolution.call source="Packed.of(\"text\")" parameters=(string) arguments=(provided("text") as string) return=Pack<string> kind=symbol target=of receiver=Packed instance=Pack<string>.<extension#1>.of
    /// @generic.instance source="Packed.of(\"text\")" id=Pack<string>
    /// @generic.instance source="Packed.of(\"text\")" id=Pack<string>.<extension#1>.of
    /// @generic.instance source=Packed.of id=Pack<T#2>
    /// @type.node source="\"text\"" type="text"

}

/// @generic.instance id=Pack<T#2> template=Pack arguments=(T#2)
/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)
/// @generic.instance id=Pack<string> template=Pack arguments=(string)
/// @generic.instance id=Pack<string>.<extension#1>.of template=of arguments=(string)
/// @generic.instance id=Packed<string> template=Packed arguments=(string)
"#,
        r#""#,
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Pack<out T> {
    value: T;
}

function read<T, 'a>(pack: &'a readonly Pack<T>): readonly T {
    pack.value
}

=== checked ===
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
/// @type.symbol symbol=read type=<T#2, read.'a>(&read.'a readonly Pack<T#2>) => Readonly<T#2>
/// @type.symbol symbol=read.T source=T type=T#2
/// @type.symbol symbol=read.pack source="pack: &readonly Pack<T>" type=&read.'a readonly Pack<T#2>
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    pack.value
    /// @type.node source=pack type=&read.'a readonly Pack<T#2>
    /// @type.node source=pack.value type=Readonly<T#2>
    /// @resolution.name source=pack target=read.pack
    /// @resolution.member source=pack.value receiver=&read.'a readonly Pack<T#2> type=Readonly<T#2> kind=field target_receiver=&read.'a readonly Pack<T#2> key=value target=Pack.value target_type=Readonly<T#2>
    /// @resolution.place source=pack placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=pack root=read.pack
    /// @resolution.place source=pack.value placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=pack.value root=read.pack keys=[value]
    /// @generic.instance source=pack id=Pack<T#2>

}

/// @generic.instance id=Pack<T#2> template=Pack arguments=(T#2)
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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
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

=== checked ===
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
/// @type.symbol symbol=same type=<T#2>(Readonly<T#2> | T#2, T#2) => void
/// @type.symbol symbol=same.T source=T type=T#2
/// @type.symbol symbol=same.actual source="actual: readonly T | T" type=Readonly<T#2> | T#2
/// @resolution.name source=T target=same.T
/// @resolution.name source=T target=same.T
/// @type.symbol symbol=same.expected source="expected: T" type=T#2
/// @resolution.name source=T target=same.T

    todo("same")
    /// @type.node source="todo(\"same\")" type=never
    /// @type.node source=todo type=(string | undefined?) => never
    /// @resolution.name source=todo target=error.panic.todo
    /// @resolution.call source="todo(\"same\")" parameters=(string | undefined) arguments=(provided("same") as string | undefined) return=never kind=symbol target=error.panic.todo
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
    /// @type.node source=same type=(Readonly<T#3> | T#3, T#3) => void
    /// @resolution.name source=same target=same
    /// @resolution.call source="same(pack.value, expected)" parameters=(Readonly<T#3> | T#3, T#3) arguments=(provided(pack.value) as Readonly<T#3> | T#3, provided(expected) as T#3) return=void kind=symbol target=same instance=same<T#3>
    /// @generic.instance source="same(pack.value, expected)" id=same<T#3>
    /// @type.node source=pack type=&check.'a readonly Pack<T#3>
    /// @type.node source=pack.value type=Readonly<T#3>
    /// @resolution.name source=pack target=check.pack
    /// @resolution.member source=pack.value receiver=&check.'a readonly Pack<T#3> type=Readonly<T#3> kind=field target_receiver=&check.'a readonly Pack<T#3> key=value target=Pack.value target_type=Readonly<T#3>
    /// @resolution.place source=pack placement="local" lifetime=check.'a access="readonly"
    /// @resolution.access source=pack root=check.pack
    /// @resolution.place source=pack.value placement="local" lifetime=check.'a access="readonly"
    /// @resolution.access source=pack.value root=check.pack keys=[value]
    /// @generic.instance source=pack id=Pack<T#3>
    /// @type.node source=expected type=T#3
    /// @resolution.name source=expected target=check.expected
    /// @resolution.place source=expected placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=expected root=check.expected

}

/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)
/// @generic.instance id=same<T#3> template=same arguments=(T#3)
"#,
        r#""#,
    );
}
