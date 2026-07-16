use crate::tests::{DirRows, TestSession};

#[test]
fn test_project_immutable_field_without_readonly_wrapper() {
    let session = TestSession::single(
        r#"
function label(name: string): void {}

local class Counter {
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function label(name: string): void {}

local class Counter {
    readonly name: string;

    constructor(name: string): this {
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

local class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(string) => this
/// @definition.method symbol=Counter.describe slot=describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<this, Counter.describe.L0, "readonly">) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(string) => this
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.name kind=place place=field(Counter.name) type=string
        /// @resolution.name source=name target=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<this, Counter.describe.L0, "readonly">) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=Borrowed<this, Counter.describe.L0, "readonly">

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=Borrowed<Counter, Counter.describe.L0, "readonly"> kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.describe.L0, "readonly">

    }
}
"#,
    );
}

#[test]
fn test_preserve_readonly_access_when_projecting_managed_field() {
    let session = TestSession::single(
        r#"
newtype interface Sink {
    write(value: string): void;
}

function consume(sink: Sink): void {}
function inspect(sink: readonly Sink): void {}

local class Meter {
    private sink: Sink;

    constructor(sink: Sink) {
        this.sink = sink;
    }

    leak(&readonly this): void {
        consume(this.sink)
    }

    forward(&readonly this): void {
        inspect(this.sink)
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Sink {
    write(value: string): void;
}

function consume(sink: Dynamic<Sink>): void {}
function inspect(sink: readonly Dynamic<Sink>): void {}

local class Meter {
    private sink: Dynamic<Sink>;

    constructor(sink: Dynamic<Sink>): this {
        this.sink = sink;
    }

    leak(&readonly this): void {
        consume(this.sink);
    }

    forward(&readonly this): void {
        inspect(this.sink);
    }
}

=== checked ===
newtype interface Sink {
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink nominal=true
/// @definition.method symbol=Sink.write source="write(value: string): void" slot=write type=(this: this, string) => void

    write(value: string): void;
    /// @type.symbol symbol=Sink.write source="write(value: string): void" type=(this: this, string) => void
    /// @type.symbol symbol=Sink.write.value source="value: string" type=string

}

function consume(sink: Sink): void {}
/// @type.symbol symbol=consume source="function consume(sink: Sink): void {}" type=(Dynamic<Sink>) => void
/// @type.symbol symbol=consume.sink source="sink: Sink" type=Dynamic<Sink>
/// @resolution.name source=Sink target=Sink

function inspect(sink: readonly Sink): void {}
/// @type.symbol symbol=inspect source="function inspect(sink: readonly Sink): void {}" type=(Readonly<Dynamic<Sink>>) => void
/// @type.symbol symbol=inspect.sink source="sink: readonly Sink" type=Readonly<Dynamic<Sink>>
/// @resolution.name source=Sink target=Sink

local class Meter {
/// @type.symbol symbol=Meter type=Meter
/// @definition.class symbol=Meter
/// @definition.field symbol=Meter.sink source="private sink: Sink" key=sink type=Dynamic<Sink>
/// @definition.method symbol=Meter.constructor slot=constructor role=constructor type=(Dynamic<Sink>) => this
/// @definition.method symbol=Meter.forward slot=forward type=<comptime Meter.forward.L0: Lifetime>(this: Borrowed<this, Meter.forward.L0, "readonly">) => void
/// @definition.method symbol=Meter.leak slot=leak type=<comptime Meter.leak.L0: Lifetime>(this: Borrowed<this, Meter.leak.L0, "readonly">) => void

    private sink: Sink;
    /// @type.symbol symbol=Meter.sink source="private sink: Sink" type=Dynamic<Sink>
    /// @resolution.name source=Sink target=Sink

    constructor(sink: Sink) {
    /// @type.symbol symbol=Meter.constructor type=(Dynamic<Sink>) => this
    /// @type.symbol symbol=Meter.constructor.sink source="sink: Sink" type=Dynamic<Sink>
    /// @resolution.name source=Sink target=Sink

        this.sink = sink;
        /// @resolution.receiver source=this kind=this declaration=Meter type=Meter
        /// @resolution.pattern.assign source=this.sink kind=place place=field(Meter.sink) type=Dynamic<Sink>
        /// @resolution.name source=sink target=Meter.constructor.sink

    }

    leak(&readonly this): void {
    /// @generic.template symbol=Meter.leak parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Meter.leak type=<comptime Meter.leak.L0: Lifetime>(this: Borrowed<this, Meter.leak.L0, "readonly">) => void
    /// @type.symbol symbol=Meter.leak.this source="&readonly this" type=Borrowed<this, Meter.leak.L0, "readonly">

        consume(this.sink)
        /// @resolution.name source=consume target=consume
        /// @resolution.call source=consume(this.sink) parameters=(Dynamic<Sink>) arguments=(provided(this.sink) as Dynamic<Sink>) return=void kind=symbol target=consume
        /// @resolution.member source=this.sink receiver=Borrowed<Meter, Meter.leak.L0, "readonly"> kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=Borrowed<Meter, Meter.leak.L0, "readonly">

    }

    forward(&readonly this): void {
    /// @generic.template symbol=Meter.forward parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Meter.forward type=<comptime Meter.forward.L0: Lifetime>(this: Borrowed<this, Meter.forward.L0, "readonly">) => void
    /// @type.symbol symbol=Meter.forward.this source="&readonly this" type=Borrowed<this, Meter.forward.L0, "readonly">

        inspect(this.sink)
        /// @resolution.name source=inspect target=inspect
        /// @resolution.call source=inspect(this.sink) parameters=(Readonly<Dynamic<Sink>>) arguments=(provided(this.sink) as Readonly<Dynamic<Sink>>) return=void kind=symbol target=inspect
        /// @resolution.member source=this.sink receiver=Borrowed<Meter, Meter.forward.L0, "readonly"> kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=Borrowed<Meter, Meter.forward.L0, "readonly">

    }
}
"#,
        r#"
/// @diagnostic.error code=EC209 message="argument of type 'readonly Dynamic<Sink>' is not assignable to parameter of type 'Dynamic<Sink>'"
/// @diagnostic.label line=17 column=22 span="sink" line_source="consume(this.sink)"
/// @diagnostic.related line=17 column=9 span="consume(this.sink)" line_source="consume(this.sink)" message="in this call"
"#,
    );
}

#[test]
fn test_project_readonly_field_before_later_function_declaration() {
    let session = TestSession::single(
        r#"
local class Counter {
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
local class Counter {
    readonly name: string;

    constructor(name: string): this {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name);
    }
}

function label(name: string): void {}

=== checked ===
local class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(string) => this
/// @definition.method symbol=Counter.describe slot=describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<this, Counter.describe.L0, "readonly">) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(string) => this
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.name kind=place place=field(Counter.name) type=string
        /// @resolution.name source=name target=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Counter.describe type=<comptime Counter.describe.L0: Lifetime>(this: Borrowed<this, Counter.describe.L0, "readonly">) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=Borrowed<this, Counter.describe.L0, "readonly">

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=Borrowed<Counter, Counter.describe.L0, "readonly"> kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=Borrowed<Counter, Counter.describe.L0, "readonly">

    }
}

function label(name: string): void {}
/// @type.symbol symbol=label source="function label(name: string): void {}" type=(string) => void
/// @type.symbol symbol=label.name source="name: string" type=string
"#,
    );
}
