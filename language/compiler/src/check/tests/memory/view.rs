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
/// @definition.method symbol=Counter.describe slot=describe type=<Counter.describe.'l0>(this: &Counter.describe.'l0 readonly this) => void

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
    /// @generic.template symbol=Counter.describe parameters=('l0)
    /// @type.symbol symbol=Counter.describe type=<Counter.describe.'l0>(this: &Counter.describe.'l0 readonly this) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=&Counter.describe.'l0 readonly this

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=&Counter.describe.'l0 readonly Counter kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.describe.'l0 readonly Counter

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
/// @definition.method symbol=Meter.forward slot=forward type=<Meter.forward.'l0>(this: &Meter.forward.'l0 readonly this) => void
/// @definition.method symbol=Meter.leak slot=leak type=<Meter.leak.'l0>(this: &Meter.leak.'l0 readonly this) => void

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
    /// @generic.template symbol=Meter.leak parameters=('l0)
    /// @type.symbol symbol=Meter.leak type=<Meter.leak.'l0>(this: &Meter.leak.'l0 readonly this) => void
    /// @type.symbol symbol=Meter.leak.this source="&readonly this" type=&Meter.leak.'l0 readonly this

        consume(this.sink)
        /// @resolution.name source=consume target=consume
        /// @resolution.call source=consume(this.sink) parameters=(Dynamic<Sink>) arguments=(provided(this.sink) as Dynamic<Sink>) return=void kind=symbol target=consume
        /// @resolution.member source=this.sink receiver=&Meter.leak.'l0 readonly Meter kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=&Meter.leak.'l0 readonly Meter

    }

    forward(&readonly this): void {
    /// @generic.template symbol=Meter.forward parameters=('l0)
    /// @type.symbol symbol=Meter.forward type=<Meter.forward.'l0>(this: &Meter.forward.'l0 readonly this) => void
    /// @type.symbol symbol=Meter.forward.this source="&readonly this" type=&Meter.forward.'l0 readonly this

        inspect(this.sink)
        /// @resolution.name source=inspect target=inspect
        /// @resolution.call source=inspect(this.sink) parameters=(Readonly<Dynamic<Sink>>) arguments=(provided(this.sink) as Readonly<Dynamic<Sink>>) return=void kind=symbol target=inspect
        /// @resolution.member source=this.sink receiver=&Meter.forward.'l0 readonly Meter kind=symbol target=Meter.sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=&Meter.forward.'l0 readonly Meter

    }
}
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'readonly Dynamic<Sink>' is not assignable to parameter of type 'Dynamic<Sink>'"
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
/// @definition.method symbol=Counter.describe slot=describe type=<Counter.describe.'l0>(this: &Counter.describe.'l0 readonly this) => void

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
    /// @generic.template symbol=Counter.describe parameters=('l0)
    /// @type.symbol symbol=Counter.describe type=<Counter.describe.'l0>(this: &Counter.describe.'l0 readonly this) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=&Counter.describe.'l0 readonly this

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=&Counter.describe.'l0 readonly Counter kind=symbol target=Counter.name
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.describe.'l0 readonly Counter

    }
}

function label(name: string): void {}
/// @type.symbol symbol=label source="function label(name: string): void {}" type=(string) => void
/// @type.symbol symbol=label.name source="name: string" type=string
"#,
    );
}

#[test]
fn test_reject_scalar_field_write_through_readonly_view() {
    let session = TestSession::single(
        r#"
struct Profile {
    count: int32;
}

declare class Person {
    profile: Profile;
}

declare const person: readonly Person;

person.profile.count = 5;
"#,
    );

    session.assert_dir_checked_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Profile {
    count: int32;
}

declare class Person {
    profile: Profile;
}

declare const person: readonly Person;

person.profile.count = 5;

=== checked ===
struct Profile {
/// @type.symbol symbol=Profile type=Profile
/// @definition.struct symbol=Profile
/// @definition.field symbol=Profile.count source="count: int32" key=count type=int32

    count: int32;
    /// @type.symbol symbol=Profile.count source="count: int32" type=int32

}

declare class Person {
/// @type.symbol symbol=Person type=Person
/// @definition.class symbol=Person
/// @definition.field symbol=Person.profile source="profile: Profile" key=profile type=Profile

    profile: Profile;
    /// @type.symbol symbol=Person.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile

}

declare const person: readonly Person;
/// @type.symbol symbol=person source=person type=Readonly<Person>
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Person target=Person

person.profile.count = 5;
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver=Readonly<Person> kind=symbol target=Person.profile
/// @resolution.pattern.assign source=person.profile.count kind=place place=field(Profile.count) type=int32
"#, r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'count'"
/// @diagnostic.label line=12 column=16 span="count" line_source="person.profile.count = 5;"
"#);
}
