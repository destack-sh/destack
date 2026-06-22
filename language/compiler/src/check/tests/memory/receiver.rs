use crate::tests::{DirRows, TestSession};

#[test]
fn test_receiver_shorthands_expand_to_this_parameter() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32;

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
        return Counter { value: 0 };
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
    value: int32;

    read(this: this): int32 {
        return this.value;
    }

    peek(this: readonly this): int32 {
        return this.value;
    }

    borrow<comptime L0: Lifetime>(this: Borrowed<this, L0, "mutable">): int32 {
        return this.value;
    }

    inspect<comptime L1: Lifetime>(this: Borrowed<this, L1, "readonly">): int32 {
        return this.value;
    }

    increment<comptime L2: Lifetime>(this: Borrowed<this, L2, "exclusive">): void {
        this.value = this.value + 1;
    }

    static zero(): Counter {
        return Counter { value: 0 };
    }
}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.read source="read(this): int32 {\n        return this.value;\n    }" slot=read type=(this: Counter) => int32
/// @definition.method symbol=Counter.peek source="peek(readonly this): int32 {\n        return this.value;\n    }" slot=peek type=(this: readonly Counter) => int32
/// @definition.method symbol=Counter.borrow source="borrow(&this): int32 {\n        return this.value;\n    }" slot=borrow type=<Counter.borrow.L0: Lifetime>(Borrowed<Counter, Counter.borrow.L0, "mutable">) => int32
/// @definition.method symbol=Counter.inspect source="inspect(&readonly this): int32 {\n        return this.value;\n    }" slot=inspect type=<Counter.inspect.L0: Lifetime>(Borrowed<Counter, Counter.inspect.L0, "readonly">) => int32
/// @definition.method symbol=Counter.increment source="increment(&exclusive this): void {\n        this.value = this.value + 1;\n    }" slot=increment type=<Counter.increment.L0: Lifetime>(Borrowed<Counter, Counter.increment.L0, "exclusive">) => void
/// @definition.method symbol=Counter.zero source="static zero(): Counter {\n        return Counter { value: 0 };\n    }" slot=zero type=() => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    read(this): int32 {
    /// @type.symbol symbol=Counter.read type=(this: Counter) => int32
    /// @type.symbol symbol=this source=this type=Counter

        return this.value;
        /// @type.node source=this.value type=int32
        /// @type.node source=this type=Counter
        /// @resolution.member source=this.value receiver=Counter kind=field key=value

    }

    peek(readonly this): int32 {
    /// @type.symbol symbol=Counter.peek type=(this: readonly Counter) => int32
    /// @type.symbol symbol=this source="readonly this" type=readonly Counter

        return this.value;
        /// @type.node source=this.value type=int32
        /// @type.node source=this type=readonly Counter
        /// @resolution.member source=this.value receiver=readonly Counter kind=field key=value

    }

    borrow(&this): int32 {
    /// @generic.template symbol=Counter.borrow parameters=(comptime L0: Lifetime origin=induced.form)
    /// @type.symbol symbol=Counter.borrow type=<Counter.borrow.L0: Lifetime>(Borrowed<Counter, Counter.borrow.L0, "mutable">) => int32
    /// @type.symbol symbol=this source="&this" type=Borrowed<Counter, Counter.borrow.L0, "mutable">

        return this.value;
        /// @type.node source=this.value type=int32
        /// @type.node source=this type=Borrowed<Counter, Counter.borrow.L0, "mutable">
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.borrow.L0, "mutable"> kind=field key=value

    }

    inspect(&readonly this): int32 {
    /// @generic.template symbol=Counter.inspect parameters=(comptime L0: Lifetime origin=induced.form)
    /// @type.symbol symbol=Counter.inspect type=<Counter.inspect.L0: Lifetime>(Borrowed<Counter, Counter.inspect.L0, "readonly">) => int32
    /// @type.symbol symbol=this source="&readonly this" type=Borrowed<Counter, Counter.inspect.L0, "readonly">

        return this.value;
        /// @type.node source=this.value type=int32
        /// @type.node source=this type=Borrowed<Counter, Counter.inspect.L0, "readonly">
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.inspect.L0, "readonly"> kind=field key=value

    }

    increment(&exclusive this): void {
    /// @generic.template symbol=Counter.increment parameters=(comptime L0: Lifetime origin=induced.form)
    /// @type.symbol symbol=Counter.increment type=<Counter.increment.L0: Lifetime>(Borrowed<Counter, Counter.increment.L0, "exclusive">) => void
    /// @type.symbol symbol=this source="&exclusive this" type=Borrowed<Counter, Counter.increment.L0, "exclusive">

        this.value = this.value + 1;
        /// @type.node source="this.value = this.value + 1" type=int32
        /// @type.node source=this.value type=int32
        /// @type.node source=this type=Borrowed<Counter, Counter.increment.L0, "exclusive">
        /// @resolution.member source=this.value receiver=Borrowed<Counter, Counter.increment.L0, "exclusive"> kind=field key=value
        /// @type.node source="this.value + 1" type=int32
        /// @type.node source=1 type=int32

    }

    static zero(): Counter {
    /// @type.symbol symbol=Counter.zero type=() => Counter

        return Counter { value: 0 };
        /// @resolution.name source=Counter target=Counter
        /// @type.node source=0 type=int32

    }
}
"#,
    );
}
