use crate::tests::{DirRows, TestSession};

/// A generic requirement is implemented by a method with matching generics, the requirement's
/// bounds read under the implemented interface's arguments.
#[test]
fn test_implement_a_generic_requirement_with_matching_generics() {
    let session = TestSession::single(
        r#"
import { Iterator } from "destack:iter";

interface Collect<T> {
    static gather<I: Iterator<T>>(values: I): this;
}

struct Bag<T> {
    first: T | undefined;
}

export extension<T> of Bag<T> implements Collect<T> {
    static gather<I: Iterator<T>>(values: I): Bag<T> {
        Bag { first: undefined }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Iterator } from "destack:iter";

interface Collect<T> {
    static gather<I: Iterator<T>>(values: I): this;
}

struct Bag<out T> {
    first: T | undefined;
}

export extension<T> of Bag<T> implements Collect<T> {
    static gather<I: Iterator<T>>(values: I): Bag<T> {
        Bag<T> { first: undefined as T | undefined }
    }
}

=== dir ===
import { Iterator } from "destack:iter";

interface Collect<T> {
/// @generic.template symbol=Collect parameters=(T#1, this: Collect<T#1>)
/// @type.symbol symbol=Collect type=Collect
/// @definition.interface symbol=Collect template=(T#1, this: Collect<T#1>)
/// @definition.where symbol=Collect relation=satisfies left=this right=Collect<T#1>
/// @definition.method symbol=Collect.gather source="static gather<I: Iterator<T>>(values: I): this" slot=gather static=true type=<I#1: Iterator<T#1>>(I#1) => this
/// @type.symbol symbol=Collect.T source=T type=T#1

    static gather<I: Iterator<T>>(values: I): this;
    /// @generic.template symbol=Collect.gather parent=template#0 parameters=(I#1: Iterator<T#1>)
    /// @type.symbol symbol=Collect.gather source="static gather<I: Iterator<T>>(values: I): this" type=<I#1: Iterator<T#1>>(I#1) => this
    /// @type.symbol symbol=Collect.gather.I source="I: Iterator<T>" type=I#1
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=Collect.T
    /// @type.symbol symbol=Collect.gather.values source="values: I" type=I#1
    /// @resolution.name source=I target=Collect.gather.I

}

struct Bag<T> {
/// @generic.template symbol=Bag parameters=(out T#2)
/// @type.symbol symbol=Bag type=Bag
/// @definition.struct symbol=Bag template=(out T#2)
/// @definition.field symbol=Bag.first source="first: T | undefined" key=first type=T#2 | undefined
/// @type.symbol symbol=Bag.T source=T type=T#2

    first: T | undefined;
    /// @type.symbol symbol=Bag.first source="first: T | undefined" type=T#2 | undefined
    /// @resolution.name source=T target=Bag.T

}

export extension<T> of Bag<T> implements Collect<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Bag<T#3>
/// @definition.implements symbol=<module>#2 source=Collect<T> target=Collect<T#3>
/// @definition.method symbol=gather slot=gather static=true type=<I#2: Iterator<T#3>>(I#2) => Bag<T#3>
/// @definition.conformance symbol=<module>#2 member=gather requirement=Collect.gather
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=T target=T
/// @resolution.name source=Collect target=Collect
/// @resolution.name source=T target=T

    static gather<I: Iterator<T>>(values: I): Bag<T> {
    /// @generic.template symbol=gather parent=template#2 parameters=(I#2: Iterator<T#3>)
    /// @type.symbol symbol=gather type=<I#2: Iterator<T#3>>(I#2) => Bag<T#3>
    /// @type.symbol symbol=gather.I source="I: Iterator<T>" type=I#2
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=T target=T
    /// @type.symbol symbol=gather.values source="values: I" type=I#2
    /// @resolution.name source=I target=gather.I
    /// @resolution.name source=Bag target=Bag
    /// @resolution.name source=T target=T

        Bag { first: undefined }
        /// @resolution.name source=Bag target=Bag

    }
}
"#,
        r#"
"#,
    );
}
