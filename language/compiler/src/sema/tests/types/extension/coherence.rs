use crate::tests::{DirRows, TestSession};

#[test]
fn test_select_a_bounded_parameter_through_its_bound_before_any_implementation() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(&readonly this): string;
}

extension of int32 implements Show {
    show(&readonly this): string {
        return "int32";
    }
}

function describe<T: Show>(value: &readonly T): string {
    return value.show();
}

declare const one: int32;

const label = describe(&readonly one);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(&readonly this): string;
}

extension of int32 implements Show {
    show(&readonly this): string {
        return "int32";
    }
}

function describe<T: Show, 'a>(value: &'a readonly T): string {
    return value.show<'a>();
}

declare const one: int32;

const label: string = describe<int32, "static">(&readonly one);

=== dir ===
newtype interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show) nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(&readonly this): string" slot=show type=<Show.show.'a>(this: &Show.show.'a readonly this) => string

    show(&readonly this): string;
    /// @generic.template symbol=Show.show parent=template#0 parameters=('a)
    /// @type.symbol symbol=Show.show source="show(&readonly this): string" type=<Show.show.'a>(this: &Show.show.'a readonly this) => string
    /// @type.symbol symbol=Show.show.this source="&readonly this" type=&Show.show.'a readonly this

}

extension of int32 implements Show {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a readonly int32) => string
/// @definition.conformance symbol=<module>#2 member=show requirement=Show.show
/// @resolution.name source=Show target=Show

    show(&readonly this): string {
    /// @generic.template symbol=show parent=template#1 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a readonly int32) => string
    /// @type.symbol symbol=show.this source="&readonly this" type=&show.'a readonly int32

        return "int32";
    }
}

function describe<T: Show>(value: &readonly T): string {
/// @generic.template symbol=describe parameters=(T: Show, 'a)
/// @type.symbol symbol=describe type=<T: Show, describe.'a>(&describe.'a readonly T) => string
/// @type.symbol symbol=describe.T source="T: Show" type=T
/// @resolution.name source=Show target=Show
/// @type.symbol symbol=describe.value source="value: &readonly T" type=&describe.'a readonly T
/// @resolution.name source=T target=describe.T

    return value.show();
    /// @resolution.name source=value target=describe.value
    /// @resolution.member source=value.show receiver=&describe.'a readonly T type=<Show.show.'a>(this: &Show.show.'a readonly T) => string kind=symbol target_receiver=&describe.'a readonly T target=Show.show
    /// @resolution.call source=value.show() parameters=() return=string regions=(describe.'a) kind=symbol target=Show.show receiver=&describe.'a readonly T instance=Show.show<describe.'a>
    /// @resolution.place source=value placement=describe.'a lifetime=describe.'a access="readonly"
    /// @resolution.access source=value root=describe.value
    /// @generic.instantiation id="Show.show<T, describe.'a>" template=Show.show arguments=(describe.'a) owner=describe

}

declare const one: int32;
/// @type.symbol symbol=one source=one type=int32
/// @resolution.pattern source=one kind=binding target=one

const label = describe(&readonly one);
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @resolution.name source=describe target=describe
/// @resolution.call source="describe(&readonly one)" parameters=(&'static readonly int32) arguments=(provided(&readonly one) as &'static readonly int32) return=string regions=("static" & "local") kind=symbol target=describe instance="describe<int32, \"static\" & \"local\">"
/// @generic.instantiation id="describe<int32, \"static\" & \"local\">" template=describe arguments=(int32, "static" & "local")
/// @resolution.name source=one target=one
/// @resolution.place source=one placement="local" lifetime="static" access="immutable"
/// @resolution.access source=one root=one
"#,
        r#"

"#,
    );
}

/// Two blanket implementations with disjoint bounds conflict, overlap read from the type alone.
#[test]
fn test_reject_two_blankets_with_disjoint_bounds() {
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

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

struct Lamp {}

extension of Lamp implements Bright {
    shine(this): string {
        return "glow";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
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

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

struct Lamp {}

extension of Lamp implements Bright {
    shine(this): string {
        return "glow";
    }
}

=== dir ===
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

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

struct Lamp {}

extension of Lamp implements Bright {
    shine(this): string {
        return "glow";
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

#[test]
fn test_reject_where_clause_blankets_that_overlap_at_one_type() {
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

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
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
        DirRows::none(),
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

struct Box<out T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
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
    shout(this): string;
}

newtype interface Bright {
    shine(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
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
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Quiet' for type 'Box<_>'"
/// @diagnostic.label line=24 column=17 span="Box" line_source="extension<T> of Box<T> implements Quiet where T: Bright {"
/// @diagnostic.related line=18 column=17 span="Box" line_source="extension<T> of Box<T> implements Quiet where T: Loud {" message="conflicting implementation"
"#,
    );
}

/// Two where-clause blanket implementations with disjoint bounds conflict by type alone.
#[test]
fn test_reject_where_clause_blankets_with_disjoint_bounds() {
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

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
    }
}

struct Bell {}

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
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

struct Box<out T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
    }
}

struct Bell {}

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}

=== dir ===
newtype interface Loud {
    shout(this): string;
}

newtype interface Bright {
    shine(this): string;
}

newtype interface Quiet {
    whisper(this): string;
}

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Quiet where T: Loud {
    whisper(this): string {
        return this.value.shout();
    }
}

extension<T> of Box<T> implements Quiet where T: Bright {
    whisper(this): string {
        return this.value.shine();
    }
}

struct Bell {}

extension of Bell implements Loud {
    shout(this): string {
        return "RING";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Quiet' for type 'Box<_>'"
/// @diagnostic.label line=24 column=17 span="Box" line_source="extension<T> of Box<T> implements Quiet where T: Bright {"
/// @diagnostic.related line=18 column=17 span="Box" line_source="extension<T> of Box<T> implements Quiet where T: Loud {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_accept_implementations_rooted_at_different_declarations() {
    let session = TestSession::single(
        r#"
newtype interface Quiet {
    whisper(this): string;
}

struct Bell {}
struct Stone {}

extension of Bell implements Quiet {
    whisper(this): string {
        return "ring";
    }
}

extension of Stone implements Quiet {
    whisper(this): string {
        return "...";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
newtype interface Quiet {
    whisper(this): string;
}

struct Bell {}
struct Stone {}

extension of Bell implements Quiet {
    whisper(this): string {
        return "ring";
    }
}

extension of Stone implements Quiet {
    whisper(this): string {
        return "...";
    }
}

=== dir ===
newtype interface Quiet {
    whisper(this): string;
}

struct Bell {}
struct Stone {}

extension of Bell implements Quiet {
    whisper(this): string {
        return "ring";
    }
}

extension of Stone implements Quiet {
    whisper(this): string {
        return "...";
    }
}
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_an_extension_member_redeclaring_an_inherent_member() {
    let session = TestSession::single(
        r#"
class Bell {
    ring(): string {
        return "inherent";
    }
}

extension of Bell {
    ring(): string {
        return "extension";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
class Bell {
    ring(): string {
        return "inherent";
    }
}

extension of Bell {
    ring(): string {
        return "extension";
    }
}

=== dir ===
class Bell {
    ring(): string {
        return "inherent";
    }
}

extension of Bell {
    ring(): string {
        return "extension";
    }
}
"#,
        r#"
/// @diagnostic.error id=inherent-member-redeclared message="member 'ring' is already declared by 'Bell'"
/// @diagnostic.label line=9 column=5 span="ring" line_source="ring(): string {"
"#,
    );
}

#[test]
fn test_reject_an_ambiguous_call_between_two_interface_requirements() {
    let session = TestSession::single(
        r#"
newtype interface Loud {
    sound(this): string;
}

newtype interface Quiet {
    sound(this): string;
}

struct Bell {}

extension of Bell implements Loud {
    sound(this): string {
        return "RING";
    }
}

extension of Bell implements Quiet {
    sound(this): string {
        return "ring";
    }
}

const bell = Bell {};
const heard = bell.sound();
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Loud {
    sound(this): string;
}

newtype interface Quiet {
    sound(this): string;
}

struct Bell {}

extension of Bell implements Loud {
    sound(this): string {
        return "RING";
    }
}

extension of Bell implements Quiet {
    sound(this): string {
        return "ring";
    }
}

const bell: Bell = Bell {};
const heard: string = bell.sound();

=== dir ===
newtype interface Loud {
/// @generic.template symbol=Loud parameters=(this: Loud)
/// @type.symbol symbol=Loud type=Loud
/// @definition.interface symbol=Loud template=(this: Loud) nominal=true
/// @definition.where symbol=Loud relation=satisfies left=this right=Loud
/// @definition.method symbol=Loud.sound source="sound(this): string" slot=sound type=(this: this) => string

    sound(this): string;
    /// @type.symbol symbol=Loud.sound source="sound(this): string" type=(this: this) => string
    /// @type.symbol symbol=Loud.sound.this source=this type=this

}

newtype interface Quiet {
/// @generic.template symbol=Quiet parameters=(this: Quiet)
/// @type.symbol symbol=Quiet type=Quiet
/// @definition.interface symbol=Quiet template=(this: Quiet) nominal=true
/// @definition.where symbol=Quiet relation=satisfies left=this right=Quiet
/// @definition.method symbol=Quiet.sound source="sound(this): string" slot=sound type=(this: this) => string

    sound(this): string;
    /// @type.symbol symbol=Quiet.sound source="sound(this): string" type=(this: this) => string
    /// @type.symbol symbol=Quiet.sound.this source=this type=this

}

struct Bell {}
/// @type.symbol symbol=Bell source="struct Bell {}" type=Bell
/// @definition.struct symbol=Bell source="struct Bell {}"

extension of Bell implements Loud {
/// @definition.extension symbol=<module>#2 form=local target=Bell
/// @definition.implements symbol=<module>#2 source=Loud target=Loud
/// @definition.method symbol=sound#1 slot=sound type=(this: Bell) => string
/// @definition.conformance symbol=<module>#2 member=sound#1 requirement=Loud.sound
/// @resolution.name source=Bell target=Bell
/// @resolution.name source=Loud target=Loud

    sound(this): string {
    /// @type.symbol symbol=sound#1 type=(this: Bell) => string
    /// @type.symbol symbol=sound.this#1 source=this type=Bell

        return "RING";
    }
}

extension of Bell implements Quiet {
/// @definition.extension symbol=<module>#3 form=local target=Bell
/// @definition.implements symbol=<module>#3 source=Quiet target=Quiet
/// @definition.method symbol=sound#2 slot=sound type=(this: Bell) => string
/// @definition.conformance symbol=<module>#3 member=sound#2 requirement=Quiet.sound
/// @resolution.name source=Bell target=Bell
/// @resolution.name source=Quiet target=Quiet

    sound(this): string {
    /// @type.symbol symbol=sound#2 type=(this: Bell) => string
    /// @type.symbol symbol=sound.this#2 source=this type=Bell

        return "ring";
    }
}

const bell = Bell {};
/// @type.symbol symbol=bell source=bell type=Bell
/// @resolution.pattern source=bell kind=binding target=bell
/// @resolution.name source=Bell target=Bell

const heard = bell.sound();
/// @type.symbol symbol=heard source=heard type=string
/// @resolution.pattern source=heard kind=binding target=heard
/// @resolution.name source=bell target=bell
/// @resolution.member source=bell.sound receiver=Bell type=(this: Bell) => string kind=symbol target_receiver=Bell target=sound#1
/// @resolution.call source=bell.sound() parameters=() return=string kind=symbol target=sound#1 receiver=Bell
/// @resolution.place source=bell placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bell root=bell
"#,
        r#"
/// @diagnostic.error id=ambiguous-member message="member 'sound' is ambiguous"
/// @diagnostic.label line=25 column=20 span="sound" line_source="const heard = bell.sound();"
"#,
    );
}

#[test]
fn test_require_the_extension_import_for_its_members() {
    let session = TestSession::builder()
        .module(
            "bell.ds",
            r#"
export struct Bell {}
"#,
        )
        .module(
            "ring.ds",
            r#"
import { Bell } from "./bell.ds";

export extension Ringing of Bell {
    ring(): string {
        return "ring";
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Bell } from "./bell.ds";

const bell = Bell {};
const sound = bell.ring();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Bell } from "./bell.ds";

const bell: Bell = Bell {};
const sound = bell.ring();

=== dir ===
import { Bell } from "./bell.ds";

const bell = Bell {};
/// @type.symbol symbol=bell source=bell type=bell.Bell
/// @resolution.pattern source=bell kind=binding target=bell
/// @resolution.name source=Bell target=bell.Bell

const sound = bell.ring();
/// @type.symbol symbol=sound source=sound type=<error>
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=bell target=bell
/// @resolution.place source=bell placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bell root=bell
/// @resolution.rejected source=bell.ring
/// @resolution.rejected source=bell.ring()
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'ring' does not exist on type 'Bell'"
/// @diagnostic.label line=5 column=20 span="ring" line_source="const sound = bell.ring();"
"#,
    );
}

#[test]
fn test_reach_extension_members_through_the_extension_import() {
    let session = TestSession::builder()
        .module(
            "bell.ds",
            r#"
export struct Bell {}
"#,
        )
        .module(
            "ring.ds",
            r#"
import { Bell } from "./bell.ds";

export extension Ringing of Bell {
    ring(): string {
        return "ring";
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Bell } from "./bell.ds";
import { Ringing } from "./ring.ds";

const bell = Bell {};
const sound = bell.ring();
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Bell } from "./bell.ds";
import { Ringing } from "./ring.ds";

const bell: Bell = Bell {};
const sound: string = bell.ring<"static">();

=== dir ===
import { Bell } from "./bell.ds";
import { Ringing } from "./ring.ds";

const bell = Bell {};
/// @type.symbol symbol=bell source=bell type=bell.Bell
/// @resolution.pattern source=bell kind=binding target=bell
/// @resolution.name source=Bell target=bell.Bell

const sound = bell.ring();
/// @type.symbol symbol=sound source=sound type=string
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=bell target=bell
/// @resolution.member source=bell.ring receiver=bell.Bell type=<ring.Ringing.ring.'a>(this: &ring.Ringing.ring.'a readonly bell.Bell) => string kind=symbol target_receiver=bell.Bell target=ring.Ringing.ring
/// @resolution.call source=bell.ring() parameters=() return=string regions=("static" & "local") kind=symbol target=ring.Ringing.ring receiver=bell.Bell adjustments=(borrow(&'static readonly bell.Bell)) instance="ring.Ringing.ring<\"static\" & \"local\">"
/// @resolution.place source=bell placement="local" lifetime="static" access="immutable"
/// @resolution.access source=bell root=bell
/// @generic.instantiation id="ring.Ringing.ring<\"static\" & \"local\">" template=ring.Ringing.ring arguments=("static" & "local")
"#,
        r#"
"#,
    );
}
