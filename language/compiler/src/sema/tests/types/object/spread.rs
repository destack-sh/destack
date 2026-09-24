use crate::tests::{DirRows, TestSession};

#[test]
fn test_let_a_later_spread_member_override_an_earlier_one() {
    let session = TestSession::single(
        r#"
interface Options {
    only: boolean;
    retries: int32;
    name?: string;
}

declare const options: Options;

const focused: Options = { ...options, only: true };
const renamed: Options = { ...options, name: "first", ...{ name: "second" } };
const literal = { ...options, only: true, retries: 3 };
const widened = { only: "yes", ...options };

interface Required {
    only: boolean;
    retries: int32;
}

declare const required: Required;

const plain: Required = { ...required, only: true };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Options {
    only: boolean;
    retries: int32;
    name?: string;
}

declare const options: Options;

const focused: Options = { ...options, only: true };
const renamed: Options = {
    ...options,
    name: "first" as string | undefined,
    ...{ name: "second" },
} as Options;
const literal: { only: boolean; retries: int64; name: string | undefined } = {
    ...options,
    only: true,
    retries: 3,
};
const widened: { only: boolean; retries: int32; name: string | undefined } = {
    only: "yes",
    ...options,
};

interface Required {
    only: boolean;
    retries: int32;
}

declare const required: Required;

const plain: Required = { ...required, only: true } as Required;

=== dir ===
interface Options {
/// @generic.template symbol=Options parameters=(this: Options)
/// @type.symbol symbol=Options type=Options
/// @definition.interface symbol=Options template=(this: Options)
/// @definition.where symbol=Options relation=satisfies left=this right=Options
/// @definition.field symbol=Options.name source="name?: string" key=name type=string
/// @definition.field symbol=Options.only source="only: boolean" key=only type=boolean
/// @definition.field symbol=Options.retries source="retries: int32" key=retries type=int32

    only: boolean;
    /// @type.symbol symbol=Options.only source="only: boolean" type=boolean

    retries: int32;
    /// @type.symbol symbol=Options.retries source="retries: int32" type=int32

    name?: string;
    /// @type.symbol symbol=Options.name source="name?: string" type=string

}

declare const options: Options;
/// @type.symbol symbol=options source=options type=Options
/// @resolution.pattern source=options kind=binding target=options
/// @resolution.name source=Options target=Options

const focused: Options = { ...options, only: true };
/// @type.symbol symbol=focused source=focused type=Options
/// @resolution.pattern source=focused kind=binding target=focused
/// @resolution.name source=Options target=Options
/// @resolution.name source=options target=options
/// @resolution.access source=options root=options

const renamed: Options = { ...options, name: "first", ...{ name: "second" } };
/// @type.symbol symbol=renamed source=renamed type=Options
/// @resolution.pattern source=renamed kind=binding target=renamed
/// @resolution.name source=Options target=Options
/// @resolution.name source=options target=options
/// @resolution.access source=options root=options

const literal = { ...options, only: true, retries: 3 };
/// @type.symbol symbol=literal source=literal type={ only: boolean; retries: int64; name: string | undefined }
/// @resolution.pattern source=literal kind=binding target=literal
/// @resolution.name source=options target=options
/// @resolution.access source=options root=options

const widened = { only: "yes", ...options };
/// @type.symbol symbol=widened source=widened type={ only: boolean; retries: int32; name: string | undefined }
/// @resolution.pattern source=widened kind=binding target=widened
/// @resolution.name source=options target=options
/// @resolution.access source=options root=options

interface Required {
/// @generic.template symbol=Required parameters=(this: Required)
/// @type.symbol symbol=Required type=Required
/// @definition.interface symbol=Required template=(this: Required)
/// @definition.where symbol=Required relation=satisfies left=this right=Required
/// @definition.field symbol=Required.only source="only: boolean" key=only type=boolean
/// @definition.field symbol=Required.retries source="retries: int32" key=retries type=int32

    only: boolean;
    /// @type.symbol symbol=Required.only source="only: boolean" type=boolean

    retries: int32;
    /// @type.symbol symbol=Required.retries source="retries: int32" type=int32

}

declare const required: Required;
/// @type.symbol symbol=required source=required type=Required
/// @resolution.pattern source=required kind=binding target=required
/// @resolution.name source=Required target=Required

const plain: Required = { ...required, only: true };
/// @type.symbol symbol=plain source=plain type=Required
/// @resolution.pattern source=plain kind=binding target=plain
/// @resolution.name source=Required target=Required
/// @resolution.name source=required target=required
/// @resolution.access source=required root=required
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ only: boolean; retries: int32; name: string | undefined }' is not assignable to type 'Options'"
/// @diagnostic.label line=10 column=26 span="{ ...options, only: true }" line_source="const focused: Options = { ...options, only: true };"
/// @diagnostic.related line=10 column=16 span="Options" line_source="const focused: Options = { ...options, only: true };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'name': expected 'string', found 'string | undefined'"
"#,
    );
}

#[test]
fn test_assign_a_required_member_into_an_optional_slot() {
    let session = TestSession::single(
        r#"
interface Named {
    only: boolean;
    name?: string;
}

declare const named: Named;
declare const full: { only: boolean; name: string };

const spreadOptional: Named = { ...named, only: true };
const literalRequired: Named = { only: true, name: "x" };
const literalAbsent: Named = { only: true };
const fromRequired: Named = full;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Named {
    only: boolean;
    name?: string;
}

declare const named: Named;
declare const full: { only: boolean; name: string };

const spreadOptional: Named = { ...named, only: true };
const literalRequired: Named = { only: true, name: "x" as string | undefined } as Named;
const literalAbsent: Named = { only: true } as Named;
const fromRequired: Named = full as Named;

=== dir ===
interface Named {
/// @generic.template symbol=Named parameters=(this: Named)
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named template=(this: Named)
/// @definition.where symbol=Named relation=satisfies left=this right=Named
/// @definition.field symbol=Named.name source="name?: string" key=name type=string
/// @definition.field symbol=Named.only source="only: boolean" key=only type=boolean

    only: boolean;
    /// @type.symbol symbol=Named.only source="only: boolean" type=boolean

    name?: string;
    /// @type.symbol symbol=Named.name source="name?: string" type=string

}

declare const named: Named;
/// @type.symbol symbol=named source=named type=Named
/// @resolution.pattern source=named kind=binding target=named
/// @resolution.name source=Named target=Named

declare const full: { only: boolean; name: string };
/// @type.symbol symbol=full source=full type={ only: boolean; name: string }
/// @resolution.pattern source=full kind=binding target=full
/// @type.symbol symbol=only source="only: boolean" type=boolean
/// @type.symbol symbol=name source="name: string" type=string

const spreadOptional: Named = { ...named, only: true };
/// @type.symbol symbol=spreadOptional source=spreadOptional type=Named
/// @resolution.pattern source=spreadOptional kind=binding target=spreadOptional
/// @resolution.name source=Named target=Named
/// @resolution.name source=named target=named
/// @resolution.access source=named root=named

const literalRequired: Named = { only: true, name: "x" };
/// @type.symbol symbol=literalRequired source=literalRequired type=Named
/// @resolution.pattern source=literalRequired kind=binding target=literalRequired
/// @resolution.name source=Named target=Named

const literalAbsent: Named = { only: true };
/// @type.symbol symbol=literalAbsent source=literalAbsent type=Named
/// @resolution.pattern source=literalAbsent kind=binding target=literalAbsent
/// @resolution.name source=Named target=Named

const fromRequired: Named = full;
/// @type.symbol symbol=fromRequired source=fromRequired type=Named
/// @resolution.pattern source=fromRequired kind=binding target=fromRequired
/// @resolution.name source=Named target=Named
/// @resolution.name source=full target=full
/// @resolution.place source=full placement="local" lifetime="static" access="immutable"
/// @resolution.access source=full root=full
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '{ only: boolean; name: string | undefined }' is not assignable to type 'Named'"
/// @diagnostic.label line=10 column=31 span="{ ...named, only: true }" line_source="const spreadOptional: Named = { ...named, only: true };"
/// @diagnostic.related line=10 column=23 span="Named" line_source="const spreadOptional: Named = { ...named, only: true };" message="expected due to this annotation"
/// @diagnostic.note message="the mismatch is in field 'name': expected 'string', found 'string | undefined'"
"#,
    );
}
