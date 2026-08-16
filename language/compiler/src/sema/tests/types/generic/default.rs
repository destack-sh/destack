use crate::tests::{DirRows, TestSession};

#[test]
fn test_default_trait_returns_this_type() {
    let session = TestSession::single(
        r#"
import { Phantom } from "destack:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Phantom } from "destack:memory";

const marker: local Phantom<int32> = Phantom<int32>.default<int32>();

=== checked ===
import { Phantom } from "destack:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
/// @type.symbol symbol=marker source=marker type=Placed<memory.phantom.Phantom<int32>, "local">
/// @resolution.name source=Phantom target=memory.phantom.Phantom
/// @type.node source=Phantom<int32> type=memory.phantom.Phantom<int32>
/// @type.node source=Phantom<int32>.default type=() => Placed<memory.phantom.Phantom<int32>, "local">
/// @type.node source=Phantom<int32>.default() type=Placed<memory.phantom.Phantom<int32>, "local">
/// @resolution.name source=Phantom target=memory.phantom.Phantom
/// @resolution.member source=Phantom<int32>.default receiver=memory.phantom.Phantom<int32> kind=symbol target=memory.phantom.default
/// @resolution.call source=Phantom<int32>.default() parameters=() return=Placed<memory.phantom.Phantom<int32>, "local"> kind=symbol target=memory.phantom.default receiver=memory.phantom.Phantom<int32> instance=memory.phantom.Phantom<int32>.<extension#1>.default
/// @resolution.instantiation source=Phantom<int32> target=memory.phantom.Phantom instance=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32> id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default() id=memory.phantom.Phantom<int32>
/// @generic.instance source=Phantom<int32>.default() id=memory.phantom.Phantom<int32>.<extension#1>.default

/// @generic.instance id=memory.phantom.Phantom<int32> template=memory.phantom.Phantom arguments=(int32)
/// @generic.instance id=memory.phantom.Phantom<int32>.<extension#1>.default template=memory.phantom.default arguments=(int32)
"#,
    );
}
