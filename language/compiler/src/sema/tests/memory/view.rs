use crate::tests::{DirRows, TestSession};

#[test]
fn test_project_immutable_field_without_readonly_wrapper() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
function label(name: string): void {}

class Counter {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name);
    }
}

=== dir ===
function label(name: string): void {}
/// @type.symbol symbol=label source="function label(name: string): void {}" type=(string) => void
/// @type.symbol symbol=label.name source="name: string" type=string

class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, string) => Counter
/// @definition.method symbol=Counter.describe slot=describe type=<Counter.describe.'a>(this: &Counter.describe.'a readonly Counter) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, string) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Counter.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=('a)
    /// @type.symbol symbol=Counter.describe type=<Counter.describe.'a>(this: &Counter.describe.'a readonly Counter) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=&Counter.describe.'a readonly Counter

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=&Counter.describe.'a readonly Counter type=string kind=field target_receiver=&Counter.describe.'a readonly Counter key=name target=Counter.name target_type=string
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.describe.'a readonly Counter
        /// @resolution.place source=this placement=Counter.describe.'a lifetime=Counter.describe.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement=Counter.describe.'a lifetime=Counter.describe.'a access="readonly"
        /// @resolution.access source=this.name root=this keys=[name]

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

class Meter {
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Sink {
    write(value: string): void;
}

function consume(sink: Sink): void {}
function inspect(sink: readonly Sink): void {}

class Meter {
    private sink: Sink;

    constructor(sink: Sink) {
        this.sink = sink;
    }

    leak(&readonly this): void {
        consume(this.sink);
    }

    forward(&readonly this): void {
        inspect(this.sink);
    }
}

=== dir ===
newtype interface Sink {
/// @generic.template symbol=Sink parameters=(this: Sink)
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink template=(this: Sink) nominal=true
/// @definition.where symbol=Sink relation=satisfies left=this right=Sink
/// @definition.method symbol=Sink.write source="write(value: string): void" slot=write type=(string) => void

    write(value: string): void;
    /// @type.symbol symbol=Sink.write source="write(value: string): void" type=(string) => void
    /// @type.symbol symbol=Sink.write.value source="value: string" type=string

}

function consume(sink: Sink): void {}
/// @type.symbol symbol=consume source="function consume(sink: Sink): void {}" type=(Sink) => void
/// @type.symbol symbol=consume.sink source="sink: Sink" type=Sink
/// @resolution.name source=Sink target=Sink

function inspect(sink: readonly Sink): void {}
/// @type.symbol symbol=inspect source="function inspect(sink: readonly Sink): void {}" type=(readonly Sink) => void
/// @type.symbol symbol=inspect.sink source="sink: readonly Sink" type=readonly Sink
/// @resolution.name source=Sink target=Sink

class Meter {
/// @type.symbol symbol=Meter type=typeof Meter
/// @definition.class symbol=Meter
/// @definition.field symbol=Meter.sink source="private sink: Sink" key=sink visibility=private type=Sink
/// @definition.method symbol=Meter.constructor slot=constructor role=constructor type=(this: &'managed Meter, Sink) => Meter
/// @definition.method symbol=Meter.forward slot=forward type=<Meter.forward.'a>(this: &Meter.forward.'a readonly Meter) => void
/// @definition.method symbol=Meter.leak slot=leak type=<Meter.leak.'a>(this: &Meter.leak.'a readonly Meter) => void

    private sink: Sink;
    /// @type.symbol symbol=Meter.sink source="private sink: Sink" type=Sink
    /// @resolution.name source=Sink target=Sink

    constructor(sink: Sink) {
    /// @type.symbol symbol=Meter.constructor type=(this: &'managed Meter, Sink) => Meter
    /// @type.symbol symbol=Meter.constructor.this type=&'managed Meter
    /// @type.symbol symbol=Meter.constructor.sink source="sink: Sink" type=Sink
    /// @resolution.name source=Sink target=Sink

        this.sink = sink;
        /// @resolution.receiver source=this kind=this declaration=Meter type=&'managed Meter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.sink kind=place
        /// @resolution.place source=this.sink placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.sink root=this keys=[sink]
        /// @resolution.assignment source=this.sink write="receiver=&'managed Meter, target=field(receiver=&'managed Meter, target=Meter.sink, type=Sink), type=Sink" type=Sink
        /// @resolution.name source=sink target=Meter.constructor.sink
        /// @resolution.place source=sink placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=sink root=Meter.constructor.sink

    }

    leak(&readonly this): void {
    /// @generic.template symbol=Meter.leak parameters=('a)
    /// @type.symbol symbol=Meter.leak type=<Meter.leak.'a>(this: &Meter.leak.'a readonly Meter) => void
    /// @type.symbol symbol=Meter.leak.this source="&readonly this" type=&Meter.leak.'a readonly Meter

        consume(this.sink)
        /// @resolution.name source=consume target=consume
        /// @resolution.call source=consume(this.sink) parameters=(Sink) arguments=(provided(this.sink) as Sink) return=void kind=symbol target=consume
        /// @resolution.member source=this.sink receiver=&Meter.leak.'a readonly Meter type=Sink kind=field target_receiver=&Meter.leak.'a readonly Meter key=sink target=Meter.sink target_type=Sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=&Meter.leak.'a readonly Meter
        /// @resolution.place source=this placement=Meter.leak.'a lifetime=Meter.leak.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.sink placement=Meter.leak.'a lifetime=Meter.leak.'a access="readonly"
        /// @resolution.access source=this.sink root=this keys=[sink]

    }

    forward(&readonly this): void {
    /// @generic.template symbol=Meter.forward parameters=('a)
    /// @type.symbol symbol=Meter.forward type=<Meter.forward.'a>(this: &Meter.forward.'a readonly Meter) => void
    /// @type.symbol symbol=Meter.forward.this source="&readonly this" type=&Meter.forward.'a readonly Meter

        inspect(this.sink)
        /// @resolution.name source=inspect target=inspect
        /// @resolution.call source=inspect(this.sink) parameters=(readonly Sink) arguments=(provided(this.sink) as readonly Sink) return=void kind=symbol target=inspect
        /// @resolution.member source=this.sink receiver=&Meter.forward.'a readonly Meter type=Sink kind=field target_receiver=&Meter.forward.'a readonly Meter key=sink target=Meter.sink target_type=Sink
        /// @resolution.receiver source=this kind=this declaration=Meter type=&Meter.forward.'a readonly Meter
        /// @resolution.place source=this placement=Meter.forward.'a lifetime=Meter.forward.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.sink placement=Meter.forward.'a lifetime=Meter.forward.'a access="readonly"
        /// @resolution.access source=this.sink root=this keys=[sink]

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_project_readonly_field_before_later_function_declaration() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(&readonly this): void {
        label(this.name);
    }
}

function label(name: string): void {}

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.name source="readonly name: string" key=name type=string
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, string) => Counter
/// @definition.method symbol=Counter.describe slot=describe type=<Counter.describe.'a>(this: &Counter.describe.'a readonly Counter) => void

    readonly name: string;
    /// @type.symbol symbol=Counter.name source="readonly name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, string) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Counter.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Counter.constructor.name

    }

    describe(&readonly this): void {
    /// @generic.template symbol=Counter.describe parameters=('a)
    /// @type.symbol symbol=Counter.describe type=<Counter.describe.'a>(this: &Counter.describe.'a readonly Counter) => void
    /// @type.symbol symbol=Counter.describe.this source="&readonly this" type=&Counter.describe.'a readonly Counter

        label(this.name)
        /// @resolution.name source=label target=label
        /// @resolution.call source=label(this.name) parameters=(string) arguments=(provided(this.name) as string) return=void kind=symbol target=label
        /// @resolution.member source=this.name receiver=&Counter.describe.'a readonly Counter type=string kind=field target_receiver=&Counter.describe.'a readonly Counter key=name target=Counter.name target_type=string
        /// @resolution.receiver source=this kind=this declaration=Counter type=&Counter.describe.'a readonly Counter
        /// @resolution.place source=this placement=Counter.describe.'a lifetime=Counter.describe.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement=Counter.describe.'a lifetime=Counter.describe.'a access="readonly"
        /// @resolution.access source=this.name root=this keys=[name]

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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct Profile {
    count: int32;
}

declare class Person {
    profile: Profile;
}

declare const person: readonly Person;

person.profile.count = 5;

=== dir ===
struct Profile {
/// @type.symbol symbol=Profile type=Profile
/// @definition.struct symbol=Profile
/// @definition.field symbol=Profile.count source="count: int32" key=count type=int32

    count: int32;
    /// @type.symbol symbol=Profile.count source="count: int32" type=int32

}

declare class Person {
/// @type.symbol symbol=Person type=typeof Person
/// @definition.class symbol=Person
/// @definition.field symbol=Person.profile source="profile: Profile" key=profile type=Profile

    profile: Profile;
    /// @type.symbol symbol=Person.profile source="profile: Profile" type=Profile
    /// @resolution.name source=Profile target=Profile

}

declare const person: readonly Person;
/// @type.symbol symbol=person source=person type=readonly Person
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Person target=Person

person.profile.count = 5;
/// @resolution.name source=person target=person
/// @resolution.member source=person.profile receiver=readonly Person type=Profile kind=field target_receiver=readonly Person key=profile target=Person.profile target_type=Profile
/// @resolution.place source=person placement="local" lifetime="static" access="immutable"
/// @resolution.access source=person root=person
/// @resolution.place source=person.profile placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.profile root=person keys=[profile]
/// @resolution.pattern.assign source=person.profile.count kind=place
/// @resolution.place source=person.profile.count placement="local" lifetime="managed" access="readonly"
/// @resolution.access source=person.profile.count root=person keys=[profile, count]
/// @resolution.assignment source=person.profile.count write="receiver=readonly Profile, target=field(receiver=readonly Profile, target=Profile.count, type=int32), type=int32" type=int32
"#,
        r#"
/// @diagnostic.error id=cannot-assign-readonly-member message="cannot assign to readonly member 'count'"
/// @diagnostic.label line=12 column=16 span="count" line_source="person.profile.count = 5;"
"#,
    );
}

#[test]
fn test_interface_alias_keeps_dynamic_dispatch() {
    let session = TestSession::single(
        r#"
interface Sink {
    write(value: string): void;
}

type SinkAlias = Sink;

declare const sink: SinkAlias;
function consume(value: SinkAlias): void {}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sink {
    write(value: string): void;
}

type SinkAlias = Sink;

declare const sink: SinkAlias;
function consume(value: SinkAlias): void {}

=== dir ===
interface Sink {
/// @generic.template symbol=Sink parameters=(this: Sink)
/// @type.symbol symbol=Sink type=Sink
/// @definition.interface symbol=Sink template=(this: Sink)
/// @definition.where symbol=Sink relation=satisfies left=this right=Sink
/// @definition.method symbol=Sink.write source="write(value: string): void" slot=write type=(string) => void

    write(value: string): void;
    /// @type.symbol symbol=Sink.write source="write(value: string): void" type=(string) => void
    /// @type.symbol symbol=Sink.write.value source="value: string" type=string

}

type SinkAlias = Sink;
/// @type.symbol symbol=SinkAlias source="type SinkAlias = Sink" type=Sink
/// @definition.type symbol=SinkAlias source="type SinkAlias = Sink" value=Sink
/// @resolution.name source=Sink target=Sink

declare const sink: SinkAlias;
/// @type.symbol symbol=sink source=sink type=SinkAlias
/// @resolution.pattern source=sink kind=binding target=sink
/// @resolution.name source=SinkAlias target=SinkAlias

function consume(value: SinkAlias): void {}
/// @type.symbol symbol=consume source="function consume(value: SinkAlias): void {}" type=(SinkAlias) => void
/// @type.symbol symbol=consume.value source="value: SinkAlias" type=SinkAlias
/// @resolution.name source=SinkAlias target=SinkAlias
"#,
    );
}
