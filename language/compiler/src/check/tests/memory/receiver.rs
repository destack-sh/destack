use crate::tests::{DirRows, TestSession};

#[test]
fn test_receiver_shorthands_expand_to_this_parameter() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;

    read(this): int32 {
        return this.value;
    }

    peek(readonly this): int32 {
        return this.value;
    }

    borrow(&this): int32 {
        return this.value;
    }

    inspect(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return new Counter();
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;

    read(this): int32 {
        return this.value;
    }

    peek(readonly this): int32 {
        return this.value;
    }

    borrow(&this): int32 {
        return this.value;
    }

    inspect(&readonly this): int32 {
        return this.value;
    }

    increment(&exclusive this): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return new Counter();
    }
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32
/// @definition.method symbol=Counter.borrow slot=borrow type=<comptime Counter.borrow.L0: Lifetime>(this: Borrowed<Counter, Counter.borrow.L0, "mutable">) => int32
/// @definition.method symbol=Counter.increment slot=increment type=<comptime Counter.increment.L0: Lifetime>(this: Borrowed<Counter, Counter.increment.L0, "exclusive">) => void
/// @definition.method symbol=Counter.inspect slot=inspect type=<comptime Counter.inspect.L0: Lifetime>(this: Borrowed<Counter, Counter.inspect.L0, "readonly">) => int32
/// @definition.method symbol=Counter.peek slot=peek type=(this: Readonly<Counter>) => int32
/// @definition.method symbol=Counter.read slot=read type=(this: Counter) => int32
/// @definition.method symbol=Counter.zero slot=zero static=true type=() => Counter

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32
    /// @type.node source=0 type=0

    read(this): int32 {
    /// @type.symbol symbol=Counter.read type=(this: Counter) => int32
    /// @type.symbol symbol=Counter.read.this source=this type=this

        return this.value;
        /// @type.node source=this type=Counter
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter

    }

    peek(readonly this): int32 {
    /// @type.symbol symbol=Counter.peek type=(this: Readonly<Counter>) => int32
    /// @type.symbol symbol=Counter.peek.this source="readonly this" type=Readonly<this>

        return this.value;
        /// @type.node source=this type=Readonly<Counter>
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Readonly<Counter> kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Readonly<Counter>

    }

    borrow(&this): int32 {
    /// @generic.template symbol=Counter.borrow parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.borrow type=<comptime Counter.borrow.L0: Lifetime>(this: Borrowed<Counter, Counter.borrow.L0, "mutable">) => int32
    /// @type.symbol symbol=Counter.borrow.this source=&this type=Borrowed<this, Counter.borrow.L0, "mutable">

        return this.value;
        /// @type.node source=this type=Borrowed<Counter, Counter.borrow.L0, "mutable">
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.borrow.L0, "mutable"> kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.borrow.L0, "mutable">

    }

    inspect(&readonly this): int32 {
    /// @generic.template symbol=Counter.inspect parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.inspect type=<comptime Counter.inspect.L0: Lifetime>(this: Borrowed<Counter, Counter.inspect.L0, "readonly">) => int32
    /// @type.symbol symbol=Counter.inspect.this source="&readonly this" type=Borrowed<this, Counter.inspect.L0, "readonly">

        return this.value;
        /// @type.node source=this type=Borrowed<Counter, Counter.inspect.L0, "readonly">
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.inspect.L0, "readonly"> kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.inspect.L0, "readonly">

    }

    increment(&exclusive this): void {
    /// @generic.template symbol=Counter.increment parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.increment type=<comptime Counter.increment.L0: Lifetime>(this: Borrowed<Counter, Counter.increment.L0, "exclusive">) => void
    /// @type.symbol symbol=Counter.increment.this source="&exclusive this" type=Borrowed<this, Counter.increment.L0, "exclusive">

        this.value = this.value + 1;
        /// @type.node source="this.value = this.value + 1" type=int32
        /// @type.node source=this type=Borrowed<Counter, Counter.increment.L0, "exclusive">
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.increment.L0, "exclusive">
        /// @resolution.pattern.assign source=this.value kind=place place=field(Counter.value) type=int32
        /// @type.node source="this.value + 1" type=int32
        /// @type.node source=this type=Borrowed<Counter, Counter.increment.L0, "exclusive">
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.increment.L0, "exclusive"> kind=symbol target=Counter.value
        /// @resolution.call source="this.value + 1" parameters=() return=int32 kind=builtin builtin=binary.add
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.increment.L0, "exclusive">
        /// @type.node source=1 type=1

    }

    static zero(): Counter {
    /// @type.symbol symbol=Counter.zero type=() => Counter
    /// @resolution.name source=Counter target=Counter

        return new Counter();
        /// @type.node source="new Counter()" type=Counter
        /// @resolution.construct source="new Counter()" parameters=() return=Counter kind=class target=Counter constructor=default
        /// @resolution.name source=Counter target=Counter

    }
}
"#,
    );
}

#[test]
fn test_interface_receivers_induce_lifetimes_like_class_receivers() {
    let session = TestSession::single(
        r#"
newtype interface Sink {
    write(&readonly this, value: string): void;
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Sink {
    write(&readonly this, value: string): void;
}

=== checked ===
newtype interface Sink {
/// @generic.template symbol=Sink parameters=()
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink template=() nominal=true
/// @definition.method symbol=Sink.write source="write(&readonly this, value: string): void" slot=write type=<comptime Sink.write.L0: Lifetime>(this: Borrowed<Sink, Sink.write.L0, "readonly">, string) => void

    write(&readonly this, value: string): void;
    /// @generic.template symbol=Sink.write parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Sink.write source="write(&readonly this, value: string): void" type=<comptime Sink.write.L0: Lifetime>(this: Borrowed<Sink, Sink.write.L0, "readonly">, string) => void
    /// @type.symbol symbol=Sink.write.this source="&readonly this" type=Borrowed<this, Sink.write.L0, "readonly">
    /// @type.symbol symbol=Sink.write.value source="value: string" type=string

}
"#,
        r#""#,
    );
}
