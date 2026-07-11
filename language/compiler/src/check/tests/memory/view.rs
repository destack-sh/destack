use crate::tests::{DirRows, TestSession};

#[test]
fn test_readonly_drops_over_immutable_payloads() {
    let session = TestSession::single(
        r#"
function label(name: string): void {}

class Counter {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name)
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
function label(name: string): void {}

class Counter {
    readonly name: string;

    constructor(name: string): Counter {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name);
    }
}

=== checked ===
function label(name: string): void {}
/// @type.symbol symbol=label source="function label(name: string): void {}" type=(string) => void
/// @type.symbol symbol=label.name source="name: string" type=string

class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(string) => Counter
/// @definition.method symbol=Counter.describe slot=describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<Counter, Counter.describe.L0, "readonly">) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(string) => Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.name kind=place place=field(Counter.name) type=string
        /// @resolution.name source=name target=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<Counter, Counter.describe.L0, "readonly">) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=Borrowed<this, Counter.describe.L0, "readonly">

        label(this.name)
        /// @type.node source=label(this.name) type=void
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @type.node source=this.name type=string
        /// @resolution.member source=this.name receiver=Borrowed<Counter, Counter.describe.L0, "readonly"> kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.describe.L0, "readonly">

    }
}
"#,
        r#""#,
    );
}

#[test]
fn test_reads_never_widen_readonly_handles() {
    let session = TestSession::single(
        r#"
newtype interface Sink {
    write(value: string): void;
}

function full(sink: Sink): void {}

function forward(sink: readonly Sink): void {}

class Meter {
    private sink: Sink;

    constructor(sink: Sink) {
        this.sink = sink;
    }

    leak(&readonly this): void {
        full(this.sink)
    }

    give(&readonly this): void {
        forward(this.sink)
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
newtype interface Sink {
    write(value: string): void;
}

function full(sink: Dynamic<Sink>): void {}

function forward(sink: readonly Dynamic<Sink>): void {}

class Meter {
    private sink: Dynamic<Sink>;

    constructor(sink: Dynamic<Sink>): Meter {
        this.sink = sink;
    }

    leak(&readonly this): void {
        full(this.sink);
    }

    give(&readonly this): void {
        forward(this.sink);
    }
}

=== checked ===
newtype interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink nominal=true
/// @definition.method symbol=Sink.write source="write(value: string): void" slot=write type=(this: Sink, string) => void

    write(value: string): void;
    /// @type.symbol symbol=Sink.write source="write(value: string): void" type=(this: Sink, string) => void
    /// @type.symbol symbol=Sink.write.value source="value: string" type=string

}

function full(sink: Sink): void {}
/// @type.symbol symbol=full source="function full(sink: Sink): void {}" type=(Dynamic<Sink>) => void
/// @type.symbol symbol=full.sink source="sink: Sink" type=Dynamic<Sink>
/// @resolution.name source=Sink target=Sink

function forward(sink: readonly Sink): void {}
/// @type.symbol symbol=forward source="function forward(sink: readonly Sink): void {}" type=(Readonly<Dynamic<Sink>>) => void
/// @type.symbol symbol=forward.sink source="sink: readonly Sink" type=Readonly<Dynamic<Sink>>
/// @resolution.name source=Sink target=Sink

class Meter {
/// @type.symbol symbol=Meter type=Meter
/// @definition.class symbol=Meter
/// @definition.field symbol=Meter.sink source="private sink: Sink" key=sink type=Dynamic<Sink>
/// @definition.method symbol=Meter.constructor slot=constructor role=constructor type=(Dynamic<Sink>) => Meter
/// @definition.method symbol=Meter.give slot=give type=<comptime Meter.give.L0: Lifetime>(this: Borrowed<Meter, Meter.give.L0, "readonly">) => void
/// @definition.method symbol=Meter.leak slot=leak type=<comptime Meter.leak.L0: Lifetime>(this: Borrowed<Meter, Meter.leak.L0, "readonly">) => void

    private sink: Sink;
    /// @type.symbol symbol=Meter.sink source="private sink: Sink" type=Dynamic<Sink>
    /// @resolution.name source=Sink target=Sink

    constructor(sink: Sink) {
    /// @type.symbol symbol=Meter.constructor type=(Dynamic<Sink>) => Meter
    /// @type.symbol symbol=Meter.constructor.sink source="sink: Sink" type=Dynamic<Sink>
    /// @resolution.name source=Sink target=Sink

        this.sink = sink;
        /// @type.node source="this.sink = sink" type=Dynamic<Sink>
        /// @type.node source=this.sink type=Dynamic<Sink>
        /// @resolution.receiver source=this kind=this declaration=Meter type=Meter
        /// @resolution.pattern.assign source=this.sink kind=place place=field(Meter.sink) type=Dynamic<Sink>
        /// @resolution.name source=sink target=Meter.constructor.sink

    }

    leak(&readonly this): void {
    /// @generic.template symbol=Meter.leak parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Meter.leak type=<comptime Meter.leak.L0: Lifetime>(this: Borrowed<Meter, Meter.leak.L0, "readonly">) => void
    /// @type.symbol symbol=Meter.leak.this source="&readonly this" type=Borrowed<this, Meter.leak.L0, "readonly">

        full(this.sink)
        /// @type.node source=full(this.sink) type=void
        /// @resolution.name source=full target=full
        /// @resolution.call source=full(this.sink) parameters=(Dynamic<Sink>) arguments=(provided(this.sink) as Dynamic<Sink>) return=void kind=symbol target=full
        /// @type.node source=this.sink type=Readonly<Dynamic<Sink>>
        /// @resolution.member source=this.sink receiver=Borrowed<Meter, Meter.leak.L0, "readonly"> kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=Borrowed<Meter, Meter.leak.L0, "readonly">

    }

    give(&readonly this): void {
    /// @generic.template symbol=Meter.give parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Meter.give type=<comptime Meter.give.L0: Lifetime>(this: Borrowed<Meter, Meter.give.L0, "readonly">) => void
    /// @type.symbol symbol=Meter.give.this source="&readonly this" type=Borrowed<this, Meter.give.L0, "readonly">

        forward(this.sink)
        /// @type.node source=forward(this.sink) type=void
        /// @resolution.name source=forward target=forward
        /// @resolution.call source=forward(this.sink) parameters=(Readonly<Dynamic<Sink>>) arguments=(provided(this.sink) as Readonly<Dynamic<Sink>>) return=void kind=symbol target=forward
        /// @type.node source=this.sink type=Readonly<Dynamic<Sink>>
        /// @resolution.member source=this.sink receiver=Borrowed<Meter, Meter.give.L0, "readonly"> kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=Borrowed<Meter, Meter.give.L0, "readonly">

    }
}
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type 'readonly Dynamic<Sink>' is not assignable to parameter of type 'Dynamic<Sink>'"
/// @diagnostic.label line=18 column=19 span="sink" line_source="full(this.sink)"
/// @diagnostic.related line=18 column=9 span="full(this.sink)" line_source="full(this.sink)" message="in this call"
"#,
    );
}

#[test]
fn test_declaration_order_does_not_change_judgments() {
    let session = TestSession::single(
        r#"
class Counter {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name)
    }
}

function label(name: string): void {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
class Counter {
    readonly name: string;

    constructor(name: string): Counter {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name);
    }
}

function label(name: string): void {}

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(string) => Counter
/// @definition.method symbol=Counter.describe slot=describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<Counter, Counter.describe.L0, "readonly">) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(string) => Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @type.node source="this.name = name" type=string
        /// @type.node source=this.name type=string
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.name kind=place place=field(Counter.name) type=string
        /// @resolution.name source=name target=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<Counter, Counter.describe.L0, "readonly">) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=Borrowed<this, Counter.describe.L0, "readonly">

        label(this.name)
        /// @type.node source=label(this.name) type=void
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @type.node source=this.name type=string
        /// @resolution.member source=this.name receiver=Borrowed<Counter, Counter.describe.L0, "readonly"> kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.describe.L0, "readonly">

    }
}

function label(name: string): void {}
/// @type.symbol symbol=label source="function label(name: string): void {}" type=(string) => void
/// @type.symbol symbol=label.name source="name: string" type=string
"#,
        r#""#,
    );
}
