use crate::tests::{DirRows, TestSession};

#[test]
fn test_finite_object_assigns_to_structural_shape() {
    let session = TestSession::single(
        r#"
type Person = { name: string };

const source = { name: "Ada" };
const person: Person = source;

person.name satisfies string;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Person = { name: string };

const source: { name: string } = { name: "Ada" };
const person: Person = source;

person.name satisfies string;

=== checked ===
type Person = { name: string };
/// @type.symbol symbol=Person source="type Person = { name: string }" type={ name: string }
/// @definition.type symbol=Person source="type Person = { name: string }" value={ name: string }

const source = { name: "Ada" };
/// @type.symbol symbol=source source=source type={ name: string }
/// @resolution.pattern source=source kind=binding target=source
/// @type.node source={ name: "Ada" } type={ name: string }
/// @type.node source="\"Ada\"" type="Ada"

const person: Person = source;
/// @type.symbol symbol=person source=person type=Person reduced={ name: string }
/// @resolution.pattern source=person kind=binding target=person
/// @resolution.name source=Person target=Person
/// @type.node source=source type={ name: string }
/// @resolution.name source=source target=source
/// @resolution.place source=source placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=source root=source

person.name satisfies string;
/// @type.node source="person.name satisfies string" type=string
/// @type.node source=person type=Person reduced={ name: string }
/// @type.node source=person.name type=string
/// @resolution.name source=person target=person
/// @resolution.member source=person.name receiver={ name: string } type=string kind=field target_receiver={ name: string } key=name target_type=string
/// @resolution.place source=person placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person root=person
/// @resolution.place source=person.name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=person.name root=person keys=[name]
"#,
    );
}

#[test]
fn test_reject_undefined_union_at_exact_optional_property() {
    let session = TestSession::single(
        r#"
type Options = { retries?: int32 };

declare const maybe: int32 | undefined;

const explicit: Options = { retries: 3 };
const omitted: Options = {};
const undecided: Options = { retries: maybe };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Options = { retries?: int32 };

declare const maybe: int32 | undefined;

const explicit: Options = { retries: 3 };
const omitted: Options = {};
const undecided: Options = { retries: maybe };

=== checked ===
type Options = { retries?: int32 };
/// @type.symbol symbol=Options source="type Options = { retries?: int32 }" type={ retries?: int32 }
/// @definition.type symbol=Options source="type Options = { retries?: int32 }" value={ retries?: int32 }

declare const maybe: int32 | undefined;
/// @type.symbol symbol=maybe source=maybe type=int32 | undefined
/// @resolution.pattern source=maybe kind=binding target=maybe

const explicit: Options = { retries: 3 };
/// @type.symbol symbol=explicit source=explicit type=Options reduced={ retries?: int32 }
/// @resolution.pattern source=explicit kind=binding target=explicit
/// @resolution.name source=Options target=Options

const omitted: Options = {};
/// @type.symbol symbol=omitted source=omitted type=Options reduced={ retries?: int32 }
/// @resolution.pattern source=omitted kind=binding target=omitted
/// @resolution.name source=Options target=Options

const undecided: Options = { retries: maybe };
/// @type.symbol symbol=undecided source=undecided type=Options reduced={ retries?: int32 }
/// @resolution.pattern source=undecided kind=binding target=undecided
/// @resolution.name source=Options target=Options
/// @resolution.name source=maybe target=maybe
/// @resolution.place source=maybe placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=maybe root=maybe
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int32 | undefined' is not assignable to type 'int32'"
/// @diagnostic.label line=8 column=39 span="maybe" line_source="const undecided: Options = { retries: maybe };"
/// @diagnostic.related line=8 column=18 span="Options" line_source="const undecided: Options = { retries: maybe };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'retries': expected 'int32', found 'undefined'"
"#,
    );
}

#[test]
fn test_accept_undefined_union_at_spelled_optional_property() {
    let session = TestSession::single(
        r#"
type Options = { retries?: int32 | undefined };

declare const maybe: int32 | undefined;

const undecided: Options = { retries: maybe };
const cleared: Options = { retries: undefined };
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Options = { retries?: int32 | undefined };

declare const maybe: int32 | undefined;

const undecided: Options = { retries: maybe };
const cleared: Options = { retries: undefined as int32 | undefined };

=== checked ===
type Options = { retries?: int32 | undefined };
/// @type.symbol symbol=Options source="type Options = { retries?: int32 | undefined }" type={ retries?: int32 | undefined }
/// @definition.type symbol=Options source="type Options = { retries?: int32 | undefined }" value={ retries?: int32 | undefined }

declare const maybe: int32 | undefined;
/// @type.symbol symbol=maybe source=maybe type=int32 | undefined
/// @resolution.pattern source=maybe kind=binding target=maybe

const undecided: Options = { retries: maybe };
/// @type.symbol symbol=undecided source=undecided type=Options reduced={ retries?: int32 | undefined }
/// @resolution.pattern source=undecided kind=binding target=undecided
/// @resolution.name source=Options target=Options
/// @resolution.name source=maybe target=maybe
/// @resolution.place source=maybe placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=maybe root=maybe

const cleared: Options = { retries: undefined };
/// @type.symbol symbol=cleared source=cleared type=Options reduced={ retries?: int32 | undefined }
/// @resolution.pattern source=cleared kind=binding target=cleared
/// @resolution.name source=Options target=Options
"#,
        r#"
"#,
    );
}

#[test]
fn test_asymmetric_accessor_splits_read_and_write_types() {
    let session = TestSession::single(
        r#"
type Meter = {
    get reading(): string;
    set reading(next: string | int32);
};

declare let meter: Meter;
const shown = meter.reading;
meter.reading = 5;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Meter = {
    get reading(): string;
    set reading(next: string | int32);
};

declare let meter: Meter;
const shown: string = meter.reading;
meter.reading = 5 as string | int32;

=== checked ===
type Meter = {
/// @type.symbol symbol=Meter type={ get reading(): string; set reading(value: string | int32) }
/// @definition.type symbol=Meter value={ get reading(): string; set reading(value: string | int32) }

    get reading(): string;
    set reading(next: string | int32);
    /// @type.symbol symbol=Meter.reading.next source="next: string | int32" type=string | int32

};

declare let meter: Meter;
/// @type.symbol symbol=meter source=meter type=Meter reduced={ get reading(): string; set reading(value: string | int32) }
/// @resolution.pattern source=meter kind=binding target=meter
/// @resolution.name source=Meter target=Meter

const shown = meter.reading;
/// @type.symbol symbol=shown source=shown type=string
/// @resolution.pattern source=shown kind=binding target=shown
/// @type.node source=meter type=Meter reduced={ get reading(): string; set reading(value: string | int32) }
/// @type.node source=meter.reading type=string
/// @resolution.name source=meter target=meter
/// @resolution.member source=meter.reading receiver={ get reading(): string; set reading(value: string | int32) } type=string kind=field target_receiver={ get reading(): string; set reading(value: string | int32) } key=reading target_type=string
/// @resolution.place source=meter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=meter root=meter
/// @resolution.access source=meter.reading root=meter keys=[reading]

meter.reading = 5;
/// @type.node source="meter.reading = 5" type=5
/// @type.node source=meter type=Meter reduced={ get reading(): string; set reading(value: string | int32) }
/// @type.node source=meter.reading type=string | int32
/// @resolution.name source=meter target=meter
/// @resolution.place source=meter placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=meter root=meter
/// @resolution.pattern.assign source=meter.reading kind=place
/// @resolution.assignment source=meter.reading write="receiver={ get reading(): string; set reading(value: string | int32) }, target=field(receiver={ get reading(): string; set reading(value: string | int32) }, target=reading, type=string | int32), type=string | int32" type=string | int32
/// @type.node source=5 type=5
"#,
    );
}

#[test]
fn test_writable_shape_widens_to_readonly_but_not_back() {
    let session = TestSession::single(
        r#"
declare let mutable: { tag: string };
declare let frozen: { readonly tag: string };

const widened: { readonly tag: string } = mutable;
const narrowed: { tag: string } = frozen;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
       r#"
=== annotated ===
declare let mutable: { tag: string };
declare let frozen: { readonly tag: string };

const widened: { readonly tag: string } = mutable;
const narrowed: { tag: string } = frozen;

=== checked ===
declare let mutable: { tag: string };
/// @type.symbol symbol=mutable source=mutable type={ tag: string }
/// @resolution.pattern source=mutable kind=binding target=mutable

declare let frozen: { readonly tag: string };
/// @type.symbol symbol=frozen source=frozen type={ readonly tag: string }
/// @resolution.pattern source=frozen kind=binding target=frozen

const widened: { readonly tag: string } = mutable;
/// @type.symbol symbol=widened source=widened type={ readonly tag: string }
/// @resolution.pattern source=widened kind=binding target=widened
/// @type.node source=mutable type={ tag: string }
/// @resolution.name source=mutable target=mutable
/// @resolution.place source=mutable placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=mutable root=mutable

const narrowed: { tag: string } = frozen;
/// @type.symbol symbol=narrowed source=narrowed type={ tag: string }
/// @resolution.pattern source=narrowed kind=binding target=narrowed
/// @type.node source=frozen type={ readonly tag: string }
/// @resolution.name source=frozen target=frozen
/// @resolution.place source=frozen placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=frozen root=frozen
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ readonly tag: string }' is not assignable to type '{ tag: string }'"
/// @diagnostic.label line=6 column=35 span="frozen" line_source="const narrowed: { tag: string } = frozen;"
/// @diagnostic.related line=6 column=17 span="{ tag: string }" line_source="const narrowed: { tag: string } = frozen;" message="expected due to this annotation"
"#,
    );
}
