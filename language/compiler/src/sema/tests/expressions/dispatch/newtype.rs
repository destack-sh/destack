use crate::tests::{DirRows, TestSession};

#[test]
fn test_await_unwraps_the_newtype_backing() {
    let session = TestSession::single(
        r#"
import { Promise } from "destack:async";

newtype Wrapper<T: Copy> = Promise<T>;

extension<T: Copy> of Wrapper<T> {
    async take(): Promise<T> {
        const value = await this;
        value
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "destack:async";

newtype Wrapper<in out T: Copy> = Promise<T>;

extension<T: Copy> of Wrapper<T> {
    async take(): Promise<T> {
        const value: T = await this;
        value
    }
}

=== dir ===
import { Promise } from "destack:async";

newtype Wrapper<T: Copy> = Promise<T>;
/// @generic.template symbol=Wrapper parameters=(in out T#1: Copy)
/// @type.symbol symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" type=Wrapper
/// @definition.newtype symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" template=(in out T#1: Copy) backing=Promise<T#1> constructors=[<T#1: Copy>(Promise<T#1>) => Wrapper<T#1>]
/// @type.symbol symbol=Wrapper.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Promise target=async.promise.Promise
/// @resolution.name source=T target=Wrapper.T

extension<T: Copy> of Wrapper<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Copy)
/// @definition.extension symbol=<module>#2 form=local target=Wrapper<T#2>
/// @definition.method symbol=take slot=take type=async (this: this) => Promise<T#2>
/// @type.symbol symbol=T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=memory.capability.Copy
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=T

    async take(): Promise<T> {
    /// @type.symbol symbol=take type=async (this: this) => Promise<T#2>
    /// @resolution.name source=Promise target=async.promise.Promise
    /// @resolution.name source=T target=T

        const value = await this;
        /// @type.symbol symbol=take.value source=value type=T#2
        /// @resolution.pattern source=value kind=binding target=take.value
        /// @type.node source="await this" type=T#2
        /// @type.node source=this type=Wrapper<T#2>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Wrapper<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

        value
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=take.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="readonly"
        /// @resolution.access source=value root=take.value

    }
}
"#,
    );
}
