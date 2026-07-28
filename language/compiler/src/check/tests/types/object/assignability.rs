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
