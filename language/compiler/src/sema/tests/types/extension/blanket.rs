use crate::tests::{DirRows, TestSession};

#[test]
fn test_blanket_extension_implements_an_interface_for_bounded_types() {
    let session = TestSession::single(
        r#"
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Horn {}

extension of Horn implements Loud {
    shout(this): string {
        return "HONK";
    }
}

const horn = Horn {};
const sound = horn.whisper();
sound satisfies string;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Horn {}

extension of Horn implements Loud {
    shout(this): string {
        return "HONK";
    }
}

const horn: Horn = Horn {};
const sound: string = horn.whisper<Horn>();
sound satisfies string;

=== dir ===
newtype interface Loud {
/// @generic.template symbol=Loud parameters=(this: Loud)
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud template=(this: Loud) nominal=true
/// @definition.where symbol=Loud relation=satisfies left=this right=Loud
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: this) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @generic.template symbol=Quiet parameters=(this: Quiet)
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet template=(this: Quiet) nominal=true
/// @definition.where symbol=Quiet relation=satisfies left=this right=Quiet
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: this) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper slot=whisper type=(this: I) => string
/// @definition.conformance symbol=<module>#2 member=whisper requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper type=(this: I) => string
    /// @type.symbol symbol=whisper.this source=this type=I

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Loud.shout<I> template=Loud.shout arguments=() owner=whisper
        /// @generic.instance id=Loud.shout<I> template=Loud.shout arguments=()

    }
}

struct Horn {}
/// @type.symbol symbol=Horn source="struct Horn {}" type=Horn
/// @definition.struct symbol=Horn source="struct Horn {}"

extension of Horn implements Loud {
/// @definition.extension symbol=<module>#3 form=local target=Horn
/// @definition.implements symbol=<module>#3 source=Loud target=Loud
/// @definition.method symbol=shout slot=shout type=(this: Horn) => string
/// @definition.conformance symbol=<module>#3 member=shout requirement=Loud.shout
/// @resolution.name source=Horn target=Horn
/// @resolution.name source=Loud target=Loud

    shout(this): string {
    /// @type.symbol symbol=shout type=(this: Horn) => string
    /// @type.symbol symbol=shout.this source=this type=Horn

        return "HONK";
    }
}

const horn = Horn {};
/// @type.symbol symbol=horn source=horn type=Horn
/// @resolution.pattern source=horn kind=binding target=horn
/// @resolution.name source=Horn target=Horn

const sound = horn.whisper();
/// @type.symbol symbol=sound source=sound type=string
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=horn target=horn
/// @resolution.member source=horn.whisper receiver=Horn type=(this: Horn) => string kind=symbol target_receiver=Horn target=whisper
/// @resolution.call source=horn.whisper() parameters=() return=string kind=symbol target=whisper receiver=Horn instance=Horn.<extension#1>.whisper
/// @resolution.place source=horn placement="local" lifetime="static" access="immutable"
/// @resolution.access source=horn root=horn
/// @generic.instantiation id=whisper<Horn> template=whisper arguments=(Horn)
/// @generic.instance id=whisper<Horn> template=whisper arguments=(Horn)

sound satisfies string;
/// @resolution.name source=sound target=sound
/// @resolution.place source=sound placement="local" lifetime="static" access="immutable"
/// @resolution.access source=sound root=sound
"#,
    );
}

#[test]
fn test_reject_a_concrete_implementation_overlapping_a_blanket() {
    let session = TestSession::single(
        r#"
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Bell {}

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

extension of Bell implements Quiet {
    whisper(this): string {
        return "ring";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Bell {}

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

extension of Bell implements Quiet {
    whisper(this): string {
        return "ring";
    }
}

=== dir ===
newtype interface Loud {
/// @generic.template symbol=Loud parameters=(this: Loud)
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud template=(this: Loud) nominal=true
/// @definition.where symbol=Loud relation=satisfies left=this right=Loud
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: this) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @generic.template symbol=Quiet parameters=(this: Quiet)
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet template=(this: Quiet) nominal=true
/// @definition.where symbol=Quiet relation=satisfies left=this right=Quiet
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: this) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper#1 slot=whisper type=(this: I) => string
/// @definition.conformance symbol=<module>#2 member=whisper#1 requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper#1 type=(this: I) => string
    /// @type.symbol symbol=whisper.this#1 source=this type=I

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Loud.shout<I> template=Loud.shout arguments=() owner=whisper#1

    }
}

struct Bell {}
/// @type.symbol symbol=Bell source="struct Bell {}" type=Bell
/// @definition.struct symbol=Bell source="struct Bell {}"

extension of Bell implements Loud {
/// @definition.extension symbol=<module>#3 form=local target=Bell
/// @definition.implements symbol=<module>#3 source=Loud target=Loud
/// @definition.method symbol=shout slot=shout type=(this: Bell) => string
/// @definition.conformance symbol=<module>#3 member=shout requirement=Loud.shout
/// @resolution.name source=Bell target=Bell
/// @resolution.name source=Loud target=Loud

    shout(this): string {
    /// @type.symbol symbol=shout type=(this: Bell) => string
    /// @type.symbol symbol=shout.this source=this type=Bell

        return "RING";
    }
}

extension of Bell implements Quiet {
/// @definition.extension symbol=<module>#4 form=local target=Bell
/// @definition.implements symbol=<module>#4 source=Quiet target=Quiet
/// @definition.method symbol=whisper#2 slot=whisper type=(this: Bell) => string
/// @definition.conformance symbol=<module>#4 member=whisper#2 requirement=Quiet.whisper
/// @resolution.name source=Bell target=Bell
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper#2 type=(this: Bell) => string
    /// @type.symbol symbol=whisper.this#2 source=this type=Bell

        return "ring";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Quiet' for type 'Bell'"
/// @diagnostic.label line=24 column=14 span="Bell" line_source="extension of Bell implements Quiet {"
/// @diagnostic.related line=10 column=23 span="I" line_source="extension<I: Loud> of I implements Quiet {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_blanket_extension_requires_its_bound() {
    let session = TestSession::single(
        r#"
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Stone {}

const stone = Stone {};
const sound = stone.whisper();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Loud {
    shout(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<I: Loud> of I implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

struct Stone {}

const stone: Stone = Stone {};
const sound = stone.whisper();

=== dir ===
newtype interface Loud {
/// @generic.template symbol=Loud parameters=(this: Loud)
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud template=(this: Loud) nominal=true
/// @definition.where symbol=Loud relation=satisfies left=this right=Loud
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: this) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @generic.template symbol=Quiet parameters=(this: Quiet)
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet template=(this: Quiet) nominal=true
/// @definition.where symbol=Quiet relation=satisfies left=this right=Quiet
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: this) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper slot=whisper type=(this: I) => string
/// @definition.conformance symbol=<module>#2 member=whisper requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper type=(this: I) => string
    /// @type.symbol symbol=whisper.this source=this type=I

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Loud.shout<I> template=Loud.shout arguments=() owner=whisper

    }
}

struct Stone {}
/// @type.symbol symbol=Stone source="struct Stone {}" type=Stone
/// @definition.struct symbol=Stone source="struct Stone {}"

const stone = Stone {};
/// @type.symbol symbol=stone source=stone type=Stone
/// @resolution.pattern source=stone kind=binding target=stone
/// @resolution.name source=Stone target=Stone

const sound = stone.whisper();
/// @type.symbol symbol=sound source=sound type=<error>
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=stone target=stone
/// @resolution.place source=stone placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stone root=stone
/// @resolution.rejected source=stone.whisper
/// @resolution.rejected source=stone.whisper()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'whisper' does not exist on type 'Stone'"
/// @diagnostic.label line=19 column=21 span="whisper" line_source="const sound = stone.whisper();"
"#,
    );
}

#[test]
fn test_project_associated_types_through_blanket_bounds() {
    let session = TestSession::single(
        r#"
struct Outcome<T, E> {
    value: T | undefined = undefined;
    failure: E | undefined = undefined;
}

newtype interface Parse<T> {
    type Failure = string;

    static parse(value: T): Outcome<this, this.Failure>;
}

newtype interface ParseInto<T> {
    type Failure = string;

    parseInto(this): Outcome<T, this.Failure>;
}

extension<T, U> of T implements ParseInto<U> where U: Parse<T> {
    type Failure = U.Failure;

    parseInto(this): Outcome<U, U.Failure> {
        return U.parse(this);
    }
}

struct Flag {}

extension of Flag implements Parse<int32> {
    type Failure = boolean;

    static parse(value: int32): Outcome<Flag, boolean> {
        return Outcome<Flag, boolean> {};
    }
}

struct Tag {}

extension of Tag implements Parse<int32> {
    static parse(value: int32): Outcome<Tag, string> {
        return Outcome<Tag, string> {};
    }
}

function decode(value: int32): Outcome<Flag, boolean> {
    return value.parseInto();
}

function label(value: int32): Outcome<Tag, string> {
    return value.parseInto();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
struct Outcome<out T, out E> {
    value: T | undefined = undefined as T | undefined;
    failure: E | undefined = undefined as E | undefined;
}

newtype interface Parse<in T> {
    type Failure = string;

    static parse(value: T): Outcome<this, this.Failure>;
}

newtype interface ParseInto<in out T> {
    type Failure = string;

    parseInto(this): Outcome<T, this.Failure>;
}

extension<T, U> of T implements ParseInto<U> where U: Parse<T> {
    type Failure = U.Failure;

    parseInto(this): Outcome<U, U.Failure> {
        return U.parse<T>(this);
    }
}

struct Flag {}

extension of Flag implements Parse<int32> {
    type Failure = boolean;

    static parse(value: int32): Outcome<Flag, boolean> {
        return Outcome<Flag, boolean> {};
    }
}

struct Tag {}

extension of Tag implements Parse<int32> {
    static parse(value: int32): Outcome<Tag, string> {
        return Outcome<Tag, string> {};
    }
}

function decode(value: int32): Outcome<Flag, boolean> {
    return value.parseInto<int32, Flag>();
}

function label(value: int32): Outcome<Tag, string> {
    return value.parseInto<int32, Tag>();
}

=== dir ===
struct Outcome<T, E> {
/// @generic.template symbol=Outcome parameters=(out T#1, out E)
/// @type.symbol symbol=Outcome type=Outcome
/// @definition.struct symbol=Outcome template=(out T#1, out E)
/// @definition.field symbol=Outcome.failure source="failure: E | undefined = undefined" key=failure type=E | undefined
/// @definition.field symbol=Outcome.value source="value: T | undefined = undefined" key=value type=T#1 | undefined
/// @type.symbol symbol=Outcome.T source=T type=T#1
/// @type.symbol symbol=Outcome.E source=E type=E

    value: T | undefined = undefined;
    /// @type.symbol symbol=Outcome.value source="value: T | undefined = undefined" type=T#1 | undefined
    /// @resolution.name source=T target=Outcome.T

    failure: E | undefined = undefined;
    /// @type.symbol symbol=Outcome.failure source="failure: E | undefined = undefined" type=E | undefined
    /// @resolution.name source=E target=Outcome.E

}

newtype interface Parse<T> {
/// @generic.template symbol=Parse parameters=(in T#2, this: Parse<T#2>)
/// @type.symbol symbol=Parse type=Parse
/// @definition.interface symbol=Parse template=(in T#2, this: Parse<T#2>) nominal=true
/// @definition.where symbol=Parse relation=satisfies left=this right=Parse<T#2>
/// @definition.associated.type symbol=Parse.Failure source="type Failure = string" key=Failure value=string
/// @definition.method symbol=Parse.parse source="static parse(value: T): Outcome<this, this.Failure>" slot=parse static=true type=(T#2) => Outcome<this, this.Failure>
/// @type.symbol symbol=Parse.T source=T type=T#2

    type Failure = string;
    /// @type.symbol symbol=Parse.Failure source="type Failure = string" type=string

    static parse(value: T): Outcome<this, this.Failure>;
    /// @type.symbol symbol=Parse.parse source="static parse(value: T): Outcome<this, this.Failure>" type=(T#2) => Outcome<this, this.Failure>
    /// @generic.instance id="Outcome<this, this.Failure>" template=Outcome arguments=(this, this.Failure)
    /// @type.symbol symbol=Parse.parse.value source="value: T" type=T#2
    /// @resolution.name source=T target=Parse.T
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=this.Failure target=Parse.Failure

}

newtype interface ParseInto<T> {
/// @generic.template symbol=ParseInto parameters=(in out T#3, this: ParseInto<T#3>)
/// @type.symbol symbol=ParseInto type=ParseInto
/// @definition.interface symbol=ParseInto template=(in out T#3, this: ParseInto<T#3>) nominal=true
/// @definition.where symbol=ParseInto relation=satisfies left=this right=ParseInto<T#3>
/// @definition.associated.type symbol=ParseInto.Failure source="type Failure = string" key=Failure value=string
/// @definition.method symbol=ParseInto.parseInto source="parseInto(this): Outcome<T, this.Failure>" slot=parseInto type=(this: this) => Outcome<T#3, this.Failure>
/// @type.symbol symbol=ParseInto.T source=T type=T#3

    type Failure = string;
    /// @type.symbol symbol=ParseInto.Failure source="type Failure = string" type=string

    parseInto(this): Outcome<T, this.Failure>;
    /// @type.symbol symbol=ParseInto.parseInto source="parseInto(this): Outcome<T, this.Failure>" type=(this: this) => Outcome<T#3, this.Failure>
    /// @generic.instance id="Outcome<T#3, this.Failure>" template=Outcome arguments=(T#3, this.Failure)
    /// @type.symbol symbol=ParseInto.parseInto.this source=this type=this
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=T target=ParseInto.T
    /// @resolution.name source=this.Failure target=ParseInto.Failure

}

extension<T, U> of T implements ParseInto<U> where U: Parse<T> {
/// @generic.template symbol=<module>#2 parameters=(T#4, U)
/// @generic.instance id=ParseInto<U> template=ParseInto arguments=(U)
/// @definition.extension symbol=<module>#2 form=local target=T#4
/// @definition.where symbol=<module>#2 source="U: Parse<T>" relation=satisfies left=U right=Parse<T#4>
/// @definition.implements symbol=<module>#2 source=ParseInto<U> target=ParseInto<U>
/// @definition.associated.type symbol=Failure#1 source="type Failure = U.Failure" key=Failure value=U.Failure
/// @definition.method symbol=parseInto slot=parseInto type=(this: T#4) => Outcome<U, U.Failure>
/// @definition.conformance symbol=<module>#2 member=Failure#1 requirement=ParseInto.Failure
/// @definition.conformance symbol=<module>#2 member=parseInto requirement=ParseInto.parseInto
/// @type.symbol symbol=T source=T type=T#4
/// @type.symbol symbol=U source=U type=U
/// @resolution.name source=T target=T
/// @resolution.name source=ParseInto target=ParseInto
/// @resolution.name source=U target=U
/// @resolution.name source=U target=U
/// @resolution.name source=Parse target=Parse
/// @generic.instance id=Parse<T#4> template=Parse arguments=(T#4)
/// @resolution.name source=T target=T

    type Failure = U.Failure;
    /// @type.symbol symbol=Failure#1 source="type Failure = U.Failure" type=U.Failure
    /// @resolution.name source=U.Failure target=U
    /// @resolution.path source=U.Failure index=1 target=Parse.Failure

    parseInto(this): Outcome<U, U.Failure> {
    /// @type.symbol symbol=parseInto type=(this: T#4) => Outcome<U, U.Failure>
    /// @generic.instance id="Outcome<U, U.Failure>" template=Outcome arguments=(U, U.Failure)
    /// @type.symbol symbol=parseInto.this source=this type=T#4
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=U target=U
    /// @resolution.name source=U.Failure target=U
    /// @resolution.path source=U.Failure index=1 target=Parse.Failure

        return U.parse(this);
        /// @resolution.name source=U target=U
        /// @resolution.member source=U.parse receiver=U type=(T#4) => Outcome<U, U.Failure> kind=symbol target_receiver=U target=Parse.parse
        /// @resolution.call source=U.parse(this) parameters=(T#4) arguments=(provided(this) as T#4) return=Outcome<U, U.Failure> kind=symbol target=Parse.parse instance=Parse<T#4>.parse
        /// @generic.instantiation id="Parse.parse<U, T#4>" template=Parse.parse arguments=(T#4) owner=parseInto
        /// @generic.instantiation id=Parse.parse<T#4> template=Parse.parse arguments=(T#4) owner=parseInto
        /// @generic.instance id="Parse.parse<U, T#4>" template=Parse.parse arguments=(T#4)
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=T#4
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}

struct Flag {}
/// @type.symbol symbol=Flag source="struct Flag {}" type=Flag
/// @definition.struct symbol=Flag source="struct Flag {}"

extension of Flag implements Parse<int32> {
/// @generic.instance id=Parse<int32> template=Parse arguments=(int32)
/// @definition.extension symbol=<module>#3 form=local target=Flag
/// @definition.implements symbol=<module>#3 source=Parse<int32> target=Parse<int32>
/// @definition.associated.type symbol=Failure#2 source="type Failure = boolean" key=Failure value=boolean
/// @definition.method symbol=parse#1 slot=parse static=true type=(int32) => Outcome<Flag, boolean>
/// @definition.conformance symbol=<module>#3 member=Failure#2 requirement=Parse.Failure
/// @definition.conformance symbol=<module>#3 member=parse#1 requirement=Parse.parse
/// @resolution.name source=Flag target=Flag
/// @resolution.name source=Parse target=Parse

    type Failure = boolean;
    /// @type.symbol symbol=Failure#2 source="type Failure = boolean" type=boolean

    static parse(value: int32): Outcome<Flag, boolean> {
    /// @type.symbol symbol=parse#1 type=(int32) => Outcome<Flag, boolean>
    /// @generic.instance id="Outcome<Flag, boolean>" template=Outcome arguments=(Flag, boolean)
    /// @type.symbol symbol=parse.value#1 source="value: int32" type=int32
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=Flag target=Flag

        return Outcome<Flag, boolean> {};
        /// @resolution.name source=Outcome target=Outcome
        /// @resolution.name source=Flag target=Flag

    }
}

struct Tag {}
/// @type.symbol symbol=Tag source="struct Tag {}" type=Tag
/// @definition.struct symbol=Tag source="struct Tag {}"

extension of Tag implements Parse<int32> {
/// @definition.extension symbol=<module>#4 form=local target=Tag
/// @definition.implements symbol=<module>#4 source=Parse<int32> target=Parse<int32>
/// @definition.method symbol=parse#2 slot=parse static=true type=(int32) => Outcome<Tag, string>
/// @definition.conformance symbol=<module>#4 member=Parse.Failure requirement=Parse.Failure
/// @definition.conformance symbol=<module>#4 member=parse#2 requirement=Parse.parse
/// @resolution.name source=Tag target=Tag
/// @resolution.name source=Parse target=Parse

    static parse(value: int32): Outcome<Tag, string> {
    /// @type.symbol symbol=parse#2 type=(int32) => Outcome<Tag, string>
    /// @generic.instance id="Outcome<Tag, string>" template=Outcome arguments=(Tag, string)
    /// @type.symbol symbol=parse.value#2 source="value: int32" type=int32
    /// @resolution.name source=Outcome target=Outcome
    /// @resolution.name source=Tag target=Tag

        return Outcome<Tag, string> {};
        /// @resolution.name source=Outcome target=Outcome
        /// @resolution.name source=Tag target=Tag

    }
}

function decode(value: int32): Outcome<Flag, boolean> {
/// @type.symbol symbol=decode type=(int32) => Outcome<Flag, boolean>
/// @type.symbol symbol=decode.value source="value: int32" type=int32
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=Flag target=Flag

    return value.parseInto();
    /// @resolution.name source=value target=decode.value
    /// @resolution.member source=value.parseInto receiver=int32 type=(this: int32) => Outcome<Flag, boolean> kind=symbol target_receiver=int32 target=parseInto
    /// @resolution.call source=value.parseInto() parameters=() return=Outcome<Flag, boolean> kind=symbol target=parseInto receiver=int32 instance=int32.<extension#1>.parseInto
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=decode.value
    /// @generic.instantiation id="parseInto<int32, Flag>" template=parseInto arguments=(int32, Flag)
    /// @generic.instance id="parseInto<int32, Flag>" template=parseInto arguments=(int32, Flag)

}

function label(value: int32): Outcome<Tag, string> {
/// @type.symbol symbol=label type=(int32) => Outcome<Tag, string>
/// @type.symbol symbol=label.value source="value: int32" type=int32
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=Tag target=Tag

    return value.parseInto();
    /// @resolution.name source=value target=label.value
    /// @resolution.member source=value.parseInto receiver=int32 type=(this: int32) => Outcome<Tag, string> kind=symbol target_receiver=int32 target=parseInto
    /// @resolution.call source=value.parseInto() parameters=() return=Outcome<Tag, string> kind=symbol target=parseInto receiver=int32 instance=int32.<extension#1>.parseInto
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=label.value
    /// @generic.instantiation id="parseInto<int32, Tag>" template=parseInto arguments=(int32, Tag)
    /// @generic.instance id="parseInto<int32, Tag>" template=parseInto arguments=(int32, Tag)

}
"#);
}

#[test]
fn test_project_interface_defaults_through_extension_conformances() {
    let session = TestSession::single(
        r#"
newtype interface Give<T> {
    give(this): T;
}

newtype interface Seed<T> {
    type Mark = string;

    static seed(value: T): this;
}

extension<T, U> of T implements Give<(U, U.Mark)> where U: Seed<T> {
    give(this): (U, U.Mark) {
        todo("give")
    }
}

struct Flag {}

extension of Flag implements Seed<int32> {
    type Mark = boolean;

    static seed(value: int32): Flag {
        return Flag {};
    }
}

struct Tag {}

extension of Tag implements Seed<int32> {
    static seed(value: int32): Tag {
        return Tag {};
    }
}

function mark(tag: Tag): Tag.Mark {
    return "x";
}

function label(value: int32): (Tag, string) {
    return value.give();
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
newtype interface Give<out T> {
    give(this): T;
}

newtype interface Seed<in T> {
    type Mark = string;

    static seed(value: T): this;
}

extension<T, U> of T implements Give<(U, U.Mark)> where U: Seed<T> {
    give(this): (U, U.Mark) {
        todo("give" as string | undefined)
    }
}

struct Flag {}

extension of Flag implements Seed<int32> {
    type Mark = boolean;

    static seed(value: int32): Flag {
        return Flag {};
    }
}

struct Tag {}

extension of Tag implements Seed<int32> {
    static seed(value: int32): Tag {
        return Tag {};
    }
}

function mark(tag: Tag): string {
    return "x";
}

function label(value: int32): (Tag, string) {
    return value.give<int32, Tag>();
}

=== dir ===
newtype interface Give<T> {
/// @generic.template symbol=Give parameters=(out T#1, this: Give<T#1>)
/// @type.symbol symbol=Give type=Give
/// @definition.interface symbol=Give template=(out T#1, this: Give<T#1>) nominal=true
/// @definition.where symbol=Give relation=satisfies left=this right=Give<T#1>
/// @definition.method symbol=Give.give source="give(this): T" slot=give type=(this: this) => T#1
/// @type.symbol symbol=Give.T source=T type=T#1

    give(this): T;
    /// @type.symbol symbol=Give.give source="give(this): T" type=(this: this) => T#1
    /// @type.symbol symbol=Give.give.this source=this type=this
    /// @resolution.name source=T target=Give.T

}

newtype interface Seed<T> {
/// @generic.template symbol=Seed parameters=(in T#2, this: Seed<T#2>)
/// @type.symbol symbol=Seed type=Seed
/// @definition.interface symbol=Seed template=(in T#2, this: Seed<T#2>) nominal=true
/// @definition.where symbol=Seed relation=satisfies left=this right=Seed<T#2>
/// @definition.associated.type symbol=Seed.Mark source="type Mark = string" key=Mark value=string
/// @definition.method symbol=Seed.seed source="static seed(value: T): this" slot=seed static=true type=(T#2) => this
/// @type.symbol symbol=Seed.T source=T type=T#2

    type Mark = string;
    /// @type.symbol symbol=Seed.Mark source="type Mark = string" type=string

    static seed(value: T): this;
    /// @type.symbol symbol=Seed.seed source="static seed(value: T): this" type=(T#2) => this
    /// @type.symbol symbol=Seed.seed.value source="value: T" type=T#2
    /// @resolution.name source=T target=Seed.T

}

extension<T, U> of T implements Give<(U, U.Mark)> where U: Seed<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3, U)
/// @generic.instance id="Give<(U, U.Mark)>" template=Give arguments=((U, U.Mark))
/// @definition.extension symbol=<module>#2 form=local target=T#3
/// @definition.implements symbol=<module>#2 source="Give<(U, U.Mark)>" target="Give<(U, U.Mark)>"
/// @definition.where symbol=<module>#2 source="U: Seed<T>" relation=satisfies left=U right=Seed<T#3>
/// @definition.method symbol=give slot=give type=(this: T#3) => (U, U.Mark)
/// @definition.conformance symbol=<module>#2 member=give requirement=Give.give
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=U source=U type=U
/// @resolution.name source=T target=T
/// @resolution.name source=Give target=Give
/// @resolution.name source=U target=U
/// @resolution.name source=U.Mark target=U
/// @resolution.path source=U.Mark index=1 target=Seed.Mark
/// @resolution.name source=U target=U
/// @resolution.name source=Seed target=Seed
/// @generic.instance id=Seed<T#3> template=Seed arguments=(T#3)
/// @resolution.name source=T target=T

    give(this): (U, U.Mark) {
    /// @type.symbol symbol=give type=(this: T#3) => (U, U.Mark)
    /// @type.symbol symbol=give.this source=this type=T#3
    /// @resolution.name source=U target=U
    /// @resolution.name source=U.Mark target=U
    /// @resolution.path source=U.Mark index=1 target=Seed.Mark

        todo("give")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"give\")" parameters=(string | undefined) arguments=(provided("give") as string | undefined) return=never kind=symbol target=todo

    }
}

struct Flag {}
/// @type.symbol symbol=Flag source="struct Flag {}" type=Flag
/// @definition.struct symbol=Flag source="struct Flag {}"

extension of Flag implements Seed<int32> {
/// @generic.instance id=Seed<int32> template=Seed arguments=(int32)
/// @definition.extension symbol=<module>#3 form=local target=Flag
/// @definition.implements symbol=<module>#3 source=Seed<int32> target=Seed<int32>
/// @definition.associated.type symbol=Mark source="type Mark = boolean" key=Mark value=boolean
/// @definition.method symbol=seed#1 slot=seed static=true type=(int32) => Flag
/// @definition.conformance symbol=<module>#3 member=Mark requirement=Seed.Mark
/// @definition.conformance symbol=<module>#3 member=seed#1 requirement=Seed.seed
/// @resolution.name source=Flag target=Flag
/// @resolution.name source=Seed target=Seed

    type Mark = boolean;
    /// @type.symbol symbol=Mark source="type Mark = boolean" type=boolean

    static seed(value: int32): Flag {
    /// @type.symbol symbol=seed#1 type=(int32) => Flag
    /// @type.symbol symbol=seed.value#1 source="value: int32" type=int32
    /// @resolution.name source=Flag target=Flag

        return Flag {};
        /// @resolution.name source=Flag target=Flag

    }
}

struct Tag {}
/// @type.symbol symbol=Tag source="struct Tag {}" type=Tag
/// @definition.struct symbol=Tag source="struct Tag {}"

extension of Tag implements Seed<int32> {
/// @definition.extension symbol=<module>#4 form=local target=Tag
/// @definition.implements symbol=<module>#4 source=Seed<int32> target=Seed<int32>
/// @definition.method symbol=seed#2 slot=seed static=true type=(int32) => Tag
/// @definition.conformance symbol=<module>#4 member=Seed.Mark requirement=Seed.Mark
/// @definition.conformance symbol=<module>#4 member=seed#2 requirement=Seed.seed
/// @resolution.name source=Tag target=Tag
/// @resolution.name source=Seed target=Seed

    static seed(value: int32): Tag {
    /// @type.symbol symbol=seed#2 type=(int32) => Tag
    /// @type.symbol symbol=seed.value#2 source="value: int32" type=int32
    /// @resolution.name source=Tag target=Tag

        return Tag {};
        /// @resolution.name source=Tag target=Tag

    }
}

function mark(tag: Tag): Tag.Mark {
/// @type.symbol symbol=mark type=(Tag) => string
/// @type.symbol symbol=mark.tag source="tag: Tag" type=Tag
/// @resolution.name source=Tag target=Tag
/// @resolution.name source=Tag.Mark target=Tag
/// @resolution.path source=Tag.Mark index=1 target=Seed.Mark

    return "x";
}

function label(value: int32): (Tag, string) {
/// @type.symbol symbol=label type=(int32) => (Tag, string)
/// @type.symbol symbol=label.value source="value: int32" type=int32
/// @resolution.name source=Tag target=Tag

    return value.give();
    /// @resolution.name source=value target=label.value
    /// @resolution.member source=value.give receiver=int32 type=(this: int32) => (Tag, string) kind=symbol target_receiver=int32 target=give
    /// @resolution.call source=value.give() parameters=() return=(Tag, string) kind=symbol target=give receiver=int32 instance=int32.<extension#1>.give
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=label.value
    /// @generic.instantiation id="give<int32, Tag>" template=give arguments=(int32, Tag)
    /// @generic.instance id="give<int32, Tag>" template=give arguments=(int32, Tag)

}
"#);
}

#[test]
fn test_default_associated_types_resolve_shadowing_imports() {
    let session = TestSession::single(
        r#"
import { Error, Result } from "destack:error";
import { TryFrom } from "destack:convert";

struct Token {}

extension of Token implements TryFrom<string> {
    static tryFrom(value: string): Result<Token, Error> {
        return Result.ok(Token {});
    }
}

function parse(value: string): Result<Token, Token.Error> {
    return Token.tryFrom(value);
}

declare const failure: Token.Error;
failure satisfies Error;
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
import { TryFrom } from "destack:convert";
import { Error, Result } from "destack:error";

struct Token {}

extension of Token implements TryFrom<string> {
    static tryFrom(value: string): Result<Token, Error> {
        return Result.ok<Token, Error>(Token {});
    }
}

function parse(value: string): Result<Token, Error> {
    return Token.tryFrom(value);
}

declare const failure: Error;
failure satisfies Error;

=== dir ===
import { Error, Result } from "destack:error";
import { TryFrom } from "destack:convert";

struct Token {}
/// @type.symbol symbol=Token source="struct Token {}" type=Token
/// @definition.struct symbol=Token source="struct Token {}"

extension of Token implements TryFrom<string> {
/// @generic.instance id=TryFrom<string> template=TryFrom arguments=(string)
/// @definition.extension symbol=<module>#2 form=local target=Token
/// @definition.implements symbol=<module>#2 source=TryFrom<string> target=TryFrom<string>
/// @definition.method symbol=tryFrom slot=tryFrom static=true type=(string) => Result<Token, Error>
/// @definition.conformance symbol=<module>#2 member=TryFrom.Error requirement=TryFrom.Error
/// @definition.conformance symbol=<module>#2 member=tryFrom requirement=TryFrom.tryFrom
/// @resolution.name source=Token target=Token
/// @resolution.name source=TryFrom target=TryFrom

    static tryFrom(value: string): Result<Token, Error> {
    /// @type.symbol symbol=tryFrom type=(string) => Result<Token, Error>
    /// @generic.instance id="Result<Token, Error>" template=Result arguments=(Token, Error)
    /// @generic.instance id=Err<Error> template=Err arguments=(Error)
    /// @generic.instance id=Ok<Token> template=Ok arguments=(Token)
    /// @type.symbol symbol=tryFrom.value source="value: string" type=string
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=Token target=Token
    /// @resolution.name source=Error target=Error

        return Result.ok(Token {});
        /// @resolution.name source=Result target=Result
        /// @resolution.member source=Result.ok receiver=Result type=(T#1) => Result<T#1, E#1> kind=symbol target_receiver=Result target=ok#1
        /// @resolution.call source="Result.ok(Token {})" parameters=(Token) arguments=(provided(Token {}) as Token) return=Result<Token, Error> kind=symbol target=ok#1 instance="Result<Token, Error>.<extension#1>.ok#1"
        /// @generic.instantiation id="ok#1<Token, Error>" template=ok#1 arguments=(Token, Error)
        /// @generic.instance id="ok#1<Token, Error>" template=ok#1 arguments=(Token, Error)
        /// @resolution.name source=Token target=Token

    }
}

function parse(value: string): Result<Token, Token.Error> {
/// @type.symbol symbol=parse type=(string) => Result<Token, Error>
/// @type.symbol symbol=parse.value source="value: string" type=string
/// @resolution.name source=Result target=Result
/// @resolution.name source=Token target=Token
/// @resolution.name source=Token.Error target=Token
/// @resolution.path source=Token.Error index=1 target=TryFrom.Error

    return Token.tryFrom(value);
    /// @resolution.name source=Token target=Token
    /// @resolution.member source=Token.tryFrom receiver=Token type=(string) => Result<Token, Error> kind=symbol target_receiver=Token target=tryFrom
    /// @resolution.call source=Token.tryFrom(value) parameters=(string) arguments=(provided(value) as string) return=Result<Token, Error> kind=symbol target=tryFrom
    /// @resolution.name source=value target=parse.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=parse.value

}

declare const failure: Token.Error;
/// @type.symbol symbol=failure source=failure type=Error
/// @resolution.pattern source=failure kind=binding target=failure
/// @resolution.name source=Token.Error target=Token
/// @resolution.path source=Token.Error index=1 target=TryFrom.Error

failure satisfies Error;
/// @resolution.name source=failure target=failure
/// @resolution.place source=failure placement="local" lifetime="static" access="immutable"
/// @resolution.access source=failure root=failure
/// @resolution.name source=Error target=Error
"#);
}

#[test]
fn test_reject_two_bounded_blankets_for_one_interface() {
    let session = TestSession::single(
        r#"
newtype interface Loud {
    shout(this): string;
}

newtype interface Bright {
    shine(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<T: Loud> of T implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

extension<T: Bright> of T implements Quiet {
    whisper(this): string {
        return this.shine();
    }
}

struct Bell {}

extension of Bell implements Loud, Bright {
    shout(this): string {
        return "RING";
    }

    shine(this): string {
        return "glint";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Loud {
    shout(this): string;
}

newtype interface Bright {
    shine(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

extension<T: Loud> of T implements Quiet {
    whisper(this): string {
        return this.shout();
    }
}

extension<T: Bright> of T implements Quiet {
    whisper(this): string {
        return this.shine();
    }
}

struct Bell {}

extension of Bell implements Loud, Bright {
    shout(this): string {
        return "RING";
    }

    shine(this): string {
        return "glint";
    }
}

=== dir ===
newtype interface Loud {
/// @generic.template symbol=Loud parameters=(this: Loud)
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud template=(this: Loud) nominal=true
/// @definition.where symbol=Loud relation=satisfies left=this right=Loud
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: this) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Bright {
/// @generic.template symbol=Bright parameters=(this: Bright)
/// @type.symbol symbol=Bright type=Bright
/// @definition.interface symbol=Bright template=(this: Bright) nominal=true
/// @definition.where symbol=Bright relation=satisfies left=this right=Bright
/// @definition.method symbol=Bright.shine source="shine(this): string" slot=shine type=(this: this) => string

    shine(this): string;
    /// @type.symbol symbol=Bright.shine source="shine(this): string" type=(this: this) => string
    /// @type.symbol symbol=Bright.shine.this source=this type=this

}

newtype interface Quiet {
/// @generic.template symbol=Quiet parameters=(this: Quiet)
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet template=(this: Quiet) nominal=true
/// @definition.where symbol=Quiet relation=satisfies left=this right=Quiet
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: this) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<T: Loud> of T implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(T#1: Loud)
/// @definition.extension symbol=<module>#2 form=local target=T#1
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper#1 slot=whisper type=(this: T#1) => string
/// @definition.conformance symbol=<module>#2 member=whisper#1 requirement=Quiet.whisper
/// @type.symbol symbol=T#1 source="T: Loud" type=T#1
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=T target=T#1
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper#1 type=(this: T#1) => string
    /// @type.symbol symbol=whisper.this#1 source=this type=T#1

        return this.shout();
        /// @resolution.member source=this.shout receiver=T#1 type=(this: T#1) => string kind=symbol target_receiver=T#1 target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=T#1
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=T#1
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Loud.shout<T#1> template=Loud.shout arguments=() owner=whisper#1

    }
}

extension<T: Bright> of T implements Quiet {
/// @generic.template symbol=<module>#3 parameters=(T#2: Bright)
/// @definition.extension symbol=<module>#3 form=local target=T#2
/// @definition.implements symbol=<module>#3 source=Quiet target=Quiet
/// @definition.method symbol=whisper#2 slot=whisper type=(this: T#2) => string
/// @definition.conformance symbol=<module>#3 member=whisper#2 requirement=Quiet.whisper
/// @type.symbol symbol=T#2 source="T: Bright" type=T#2
/// @resolution.name source=Bright target=Bright
/// @resolution.name source=T target=T#2
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper#2 type=(this: T#2) => string
    /// @type.symbol symbol=whisper.this#2 source=this type=T#2

        return this.shine();
        /// @resolution.member source=this.shine receiver=T#2 type=(this: T#2) => string kind=symbol target_receiver=T#2 target=Bright.shine
        /// @resolution.call source=this.shine() parameters=() return=string kind=symbol target=Bright.shine receiver=T#2
        /// @resolution.receiver source=this kind=this declaration=<module>#3 type=T#2
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id=Bright.shine<T#2> template=Bright.shine arguments=() owner=whisper#2

    }
}

struct Bell {}
/// @type.symbol symbol=Bell source="struct Bell {}" type=Bell
/// @definition.struct symbol=Bell source="struct Bell {}"

extension of Bell implements Loud, Bright {
/// @definition.extension symbol=<module>#4 form=local target=Bell
/// @definition.implements symbol=<module>#4 source=Bright target=Bright
/// @definition.implements symbol=<module>#4 source=Loud target=Loud
/// @definition.method symbol=shine slot=shine type=(this: Bell) => string
/// @definition.method symbol=shout slot=shout type=(this: Bell) => string
/// @definition.conformance symbol=<module>#4 member=shine requirement=Bright.shine
/// @definition.conformance symbol=<module>#4 member=shout requirement=Loud.shout
/// @resolution.name source=Bell target=Bell
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=Bright target=Bright

    shout(this): string {
    /// @type.symbol symbol=shout type=(this: Bell) => string
    /// @type.symbol symbol=shout.this source=this type=Bell

        return "RING";
    }

    shine(this): string {
    /// @type.symbol symbol=shine type=(this: Bell) => string
    /// @type.symbol symbol=shine.this source=this type=Bell

        return "glint";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Quiet' for type 'T'"
/// @diagnostic.label line=20 column=25 span="T" line_source="extension<T: Bright> of T implements Quiet {"
/// @diagnostic.related line=14 column=23 span="T" line_source="extension<T: Loud> of T implements Quiet {" message="conflicting implementation"
"#,
    );
}

/// A blanket over a bounded parameter records the members implementing its interface.
#[test]
fn test_record_member_conformances_of_a_blanket_over_a_bounded_parameter() {
    let session = TestSession::single(
        r#"
import { Integer } from "destack:math";

newtype interface Dup {
    dup(&readonly this): ^this;
}

export extension<T: Integer> of T implements Dup {
    dup(&readonly this): ^this {
        *this
    }
}

const copied = (1 as int32).dup();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Integer } from "destack:math";

newtype interface Dup {
    dup(&readonly this): ^this;
}

export extension<T: Integer> of T implements Dup {
    dup(&readonly this): ^T {
        *this
    }
}

const copied: int32 = (1 as int32).dup<int32, "frame">();

=== dir ===
import { Integer } from "destack:math";

newtype interface Dup {
/// @generic.template symbol=Dup parameters=(this: Dup)
/// @type.symbol symbol=Dup type=Dup
/// @definition.interface symbol=Dup template=(this: Dup) nominal=true
/// @definition.where symbol=Dup relation=satisfies left=this right=Dup
/// @definition.method symbol=Dup.dup source="dup(&readonly this): ^this" slot=dup type=<Dup.dup.'a>(this: &Dup.dup.'a readonly this) => ^this

    dup(&readonly this): ^this;
    /// @generic.template symbol=Dup.dup parent=template#0 parameters=('a)
    /// @type.symbol symbol=Dup.dup source="dup(&readonly this): ^this" type=<Dup.dup.'a>(this: &Dup.dup.'a readonly this) => ^this
    /// @type.symbol symbol=Dup.dup.this source="&readonly this" type=&Dup.dup.'a readonly this

}

export extension<T: Integer> of T implements Dup {
/// @generic.template symbol=<module>#2 parameters=(T: Integer)
/// @definition.extension symbol=<module>#2 form=exported target=T
/// @definition.implements symbol=<module>#2 source=Dup target=Dup
/// @definition.method symbol=dup slot=dup type=<dup.'a>(this: &dup.'a readonly T) => ^T
/// @definition.conformance symbol=<module>#2 member=dup requirement=Dup.dup
/// @type.symbol symbol=T source="T: Integer" type=T
/// @resolution.name source=Integer target=Integer
/// @resolution.name source=T target=T
/// @resolution.name source=Dup target=Dup

    dup(&readonly this): ^this {
    /// @generic.template symbol=dup parent=template#1 parameters=('a)
    /// @type.symbol symbol=dup type=<dup.'a>(this: &dup.'a readonly T) => ^T
    /// @type.symbol symbol=dup.this source="&readonly this" type=&dup.'a readonly T

        *this
        /// @resolution.place source=*this placement=dup.'a lifetime=dup.'a access="readonly"
        /// @resolution.operator source=*this type=^T operator="*" kind=builtin operands=[this as &dup.'a readonly T]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&dup.'a readonly T
        /// @resolution.place source=this placement=dup.'a lifetime=dup.'a access="readonly"
        /// @resolution.access source=this root=this

    }
}

const copied = (1 as int32).dup();
/// @type.symbol symbol=copied source=copied type=int32
/// @resolution.pattern source=copied kind=binding target=copied
/// @resolution.member source="(1 as int32).dup" receiver=int32 type=<dup.'a>(this: &dup.'a readonly int32) => ^int32 kind=symbol target_receiver=int32 target=dup
/// @resolution.call source=(1 as int32).dup() parameters=() return=^int32 regions=("frame" & "local") kind=symbol target=dup receiver=int32 adjustments=(borrow(&'frame readonly int32)) instance="int32.<extension#1>.dup<\"frame\" & \"local\">"
/// @generic.instantiation id="dup<int32, \"frame\" & \"local\">" template=dup arguments=(int32, "frame" & "local")
/// @generic.instantiation id=dup<int32> template=dup arguments=(int32)
/// @generic.instance id="dup<int32, \"bound0\" & \"local\">" template=dup arguments=(int32, "bound0" & "local")
"#,
    );
}

/// A static requirement called through a parameter closes on the parameter's argument.
#[test]
fn test_close_a_static_requirement_through_a_parameter_on_its_argument() {
    let session = TestSession::single(
        r#"
import { From } from "destack:convert";

function build<E: From<string>>(residual: string): E {
    return E.from(residual);
}

const built: string = build<string>("a");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_witnesses(),
        r#"
=== annotated ===
import { From } from "destack:convert";

function build<E: From<string>>(residual: string): E {
    return E.from<string>(residual);
}

const built: string = build<string>("a");

=== dir ===
import { From } from "destack:convert";

function build<E: From<string>>(residual: string): E {
/// @generic.template symbol=build parameters=(E: From<string>)
/// @type.symbol symbol=build type=<E: From<string>>(string) => E
/// @type.symbol symbol=build.E source="E: From<string>" type=E
/// @resolution.name source=From target=From
/// @generic.instance id=From<string> template=From arguments=(string)
/// @type.symbol symbol=build.residual source="residual: string" type=string
/// @resolution.name source=E target=build.E

    return E.from(residual);
    /// @resolution.name source=E target=build.E
    /// @resolution.member source=E.from receiver=E type=(string) => E kind=symbol target_receiver=E target=From.from
    /// @resolution.call source=E.from(residual) parameters=(string) arguments=(provided(residual) as string) return=E kind=symbol target=From.from instance=From<string>.from
    /// @generic.instantiation id="From.from<E, string>" template=From.from arguments=(string) owner=build
    /// @generic.instantiation id=From.from<string> template=From.from arguments=(string) owner=build
    /// @generic.instance id="From.from<E, string>" template=From.from arguments=(string)
    /// @resolution.name source=residual target=build.residual
    /// @resolution.place source=residual placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=residual root=build.residual

}

const built: string = build<string>("a");
/// @type.symbol symbol=built source=built type=string
/// @resolution.pattern source=built kind=binding target=built
/// @resolution.name source=build target=build
/// @resolution.call source="build<string>(\"a\")" parameters=(string) arguments=(provided("a") as string) return=string kind=symbol target=build instance=build<string>
/// @generic.instantiation id=build<string> template=build arguments=(string)
/// @generic.instance id=build<string> template=build arguments=(string)
/// @generic.instance id=from<string> template=from arguments=(string)

/// @generic.witness type=string interface=From<string> functions=(From.from: from<string>)
"#,
    );
}
