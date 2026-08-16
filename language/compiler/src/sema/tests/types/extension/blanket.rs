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
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud nominal=true
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: Loud) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: Loud) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet nominal=true
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: Quiet) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: Quiet) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper slot=whisper type=(this: this) => string
/// @definition.conformance symbol=<module>#2 member=whisper requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper type=(this: this) => string
    /// @type.symbol symbol=whisper.this source=this type=this

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

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
    /// @type.symbol symbol=shout.this source=this type=this

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
/// @resolution.place source=horn placement="local" lifetime="static" access="readonly"
/// @resolution.access source=horn root=horn
/// @generic.instantiation id=whisper<Horn> template=whisper arguments=(Horn)
/// @generic.instantiation id=whisper<Horn> template=whisper arguments=(Horn)
/// @generic.instance id=whisper<Horn> template=whisper arguments=(Horn)

sound satisfies string;
/// @resolution.name source=sound target=sound
/// @resolution.place source=sound placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=sound root=sound
"#,
    );
}

#[test]
fn test_blanket_extension_yields_to_the_declared_implementation() {
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

const bell = Bell {};
const sound = bell.whisper();
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

const bell: Bell = Bell {};
const sound: string = bell.whisper();
sound satisfies string;

=== dir ===
newtype interface Loud {
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud nominal=true
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: Loud) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: Loud) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet nominal=true
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: Quiet) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: Quiet) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper#1 slot=whisper type=(this: this) => string
/// @definition.conformance symbol=<module>#2 member=whisper#1 requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper#1 type=(this: this) => string
    /// @type.symbol symbol=whisper.this#1 source=this type=this

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

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
    /// @type.symbol symbol=shout.this source=this type=this

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
    /// @type.symbol symbol=whisper.this#2 source=this type=this

        return "ring";
    }
}

const bell = Bell {};
/// @type.symbol symbol=bell source=bell type=Bell
/// @resolution.pattern source=bell kind=binding target=bell
/// @resolution.name source=Bell target=Bell

const sound = bell.whisper();
/// @type.symbol symbol=sound source=sound type=string
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=bell target=bell
/// @resolution.member source=bell.whisper receiver=Bell type=(this: Bell) => string kind=symbol target_receiver=Bell target=whisper#2
/// @resolution.call source=bell.whisper() parameters=() return=string kind=symbol target=whisper#2 receiver=Bell
/// @resolution.place source=bell placement="local" lifetime="static" access="readonly"
/// @resolution.access source=bell root=bell

sound satisfies string;
/// @resolution.name source=sound target=sound
/// @resolution.place source=sound placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=sound root=sound
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
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud nominal=true
/// @definition.method symbol=Loud.shout source="shout(this): string" slot=shout type=(this: this) => string

    shout(this): string;
    /// @type.symbol symbol=Loud.shout source="shout(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.shout.this source=this type=this

}

newtype interface Quiet {
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet nominal=true
/// @definition.method symbol=Quiet.whisper source="whisper(this): string" slot=whisper type=(this: this) => string

    whisper(this): string;
    /// @type.symbol symbol=Quiet.whisper source="whisper(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.whisper.this source=this type=this

}

extension<I: Loud> of I implements Quiet {
/// @generic.template symbol=<module>#2 parameters=(I: Loud)
/// @definition.extension symbol=<module>#2 form=local target=I
/// @definition.implements symbol=<module>#2 source=Quiet target=Quiet
/// @definition.method symbol=whisper slot=whisper type=(this: this) => string
/// @definition.conformance symbol=<module>#2 member=whisper requirement=Quiet.whisper
/// @type.symbol symbol=I source="I: Loud" type=I
/// @resolution.name source=Loud target=Loud
/// @resolution.name source=I target=I
/// @resolution.name source=Quiet target=Quiet

    whisper(this): string {
    /// @type.symbol symbol=whisper type=(this: this) => string
    /// @type.symbol symbol=whisper.this source=this type=this

        return this.shout();
        /// @resolution.member source=this.shout receiver=I type=(this: I) => string kind=symbol target_receiver=I target=Loud.shout
        /// @resolution.call source=this.shout() parameters=() return=string kind=symbol target=Loud.shout receiver=I
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=I
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

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
/// @resolution.place source=stone placement="local" lifetime="static" access="readonly"
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
