use crate::tests::{DirRows, TestSession};

#[test]
fn test_await_unwraps_the_newtype_backing() {
    let session = TestSession::single(
        r#"
import { Promise } from "tspp:async";

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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Promise } from "tspp:async";

newtype Wrapper<out T: Copy> = Promise<T>;

extension<T: Copy> of Wrapper<T> {
    async take(): Promise<T> {
        const value: T = await this;
        value
    }
}

=== dir ===
import { Promise } from "tspp:async";

newtype Wrapper<T: Copy> = Promise<T>;
/// @generic.template symbol=Wrapper parameters=(out T#1: Copy)
/// @type.symbol symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" type=Wrapper
/// @generic.instance id=Promise<T#1> template=Promise arguments=(T#1)
/// @definition.newtype symbol=Wrapper source="newtype Wrapper<T: Copy> = Promise<T>" template=(out T#1: Copy) backing=Promise<T#1> constructors=[<T#1: Copy>(Promise<T#1>) => Wrapper<T#1>]
/// @type.symbol symbol=Wrapper.T source="T: Copy" type=T#1
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Promise target=Promise
/// @resolution.name source=T target=Wrapper.T

extension<T: Copy> of Wrapper<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: Copy)
/// @generic.instance id=Wrapper<T#2> template=Wrapper arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Wrapper<T#2>
/// @definition.method symbol=take slot=take type=async (this: Wrapper<T#2>) => Promise<T#2>
/// @type.symbol symbol=T source="T: Copy" type=T#2
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=Wrapper target=Wrapper
/// @resolution.name source=T target=T

    async take(): Promise<T> {
    /// @type.symbol symbol=take type=async (this: Wrapper<T#2>) => Promise<T#2>
    /// @type.symbol symbol=take.this type=Wrapper<T#2>
    /// @resolution.call parameters=(^Function<(), T#2, "once">) arguments=(supplied(0) as ^Function<(), T#2, "once">) return=Promise<T#2> kind=symbol target=Promise.create instance=Promise.create<T#2>
    /// @generic.instantiation id=Promise.create<T#2> template=Promise.create arguments=(T#2) owner=take
    /// @generic.instance id=Promise.create<T#2> template=Promise.create arguments=(T#2)
    /// @generic.instance id=Promise.fulfill<T#2> template=Promise.fulfill arguments=(T#2)
    /// @generic.instance id=Promise.pending<T#2> template=Promise.pending arguments=(T#2)
    /// @generic.instance id=Promise.queueWaiters<T#2> template=Promise.queueWaiters arguments=(T#2)
    /// @generic.instance id=Promise.symbol12<T#2> template=Promise.symbol12 arguments=(T#2)
    /// @generic.instance id=Promise<T#2> template=Promise arguments=(T#2)
    /// @resolution.name source=Promise target=Promise
    /// @resolution.name source=T target=T

        const value = await this;
        /// @type.symbol symbol=take.value source=value type=T#2
        /// @resolution.pattern source=value kind=binding target=take.value
        /// @type.node source="await this" type=T#2
        /// @resolution.call source="await this" parameters=(Promise<T#2>) arguments=(provided(this) as Promise<T#2>) return=T#2 kind=symbol target=Promise.park receiver=Promise<T#2> instance=Promise<T#2>.park<T#2>
        /// @generic.instantiation id="Promise.park<T#2, T#2>" template=Promise.park arguments=(T#2, T#2) owner=take
        /// @generic.instance id="Promise.park<T#2, T#2>" template=Promise.park arguments=(T#2, T#2)
        /// @generic.instance id=Promise.addWaiter<T#2> template=Promise.addWaiter arguments=(T#2)
        /// @generic.instance id=Promise.observe<T#2> template=Promise.observe arguments=(T#2)
        /// @generic.instance id=Promise.queueWaiter<T#2> template=Promise.queueWaiter arguments=(T#2)
        /// @generic.instance id=PromiseAwaiter.symbol161<T#2> template=PromiseAwaiter.symbol161 arguments=(T#2)
        /// @generic.instance id=PromiseAwaiter<T#2> template=PromiseAwaiter arguments=(T#2)
        /// @generic.instance id=PromiseForwarded<T#2> template=PromiseForwarded arguments=(T#2)
        /// @generic.instance id=PromiseFulfilled<T#2> template=PromiseFulfilled arguments=(T#2)
        /// @type.node source=this type=Wrapper<T#2>
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=Wrapper<T#2>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

        value
        /// @type.node source=value type=T#2
        /// @resolution.name source=value target=take.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=take.value

    }
}
"#,
    );
}
