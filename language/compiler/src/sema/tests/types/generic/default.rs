use crate::tests::{DirRows, TestSession};

#[test]
fn test_default_trait_returns_this_type() {
    let session = TestSession::single(
        r#"
import { Phantom } from "tspp:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Phantom } from "tspp:memory";

const marker: Phantom<int32> = Phantom<int32>.default<int32>();

=== dir ===
import { Phantom } from "tspp:memory";

const marker: Phantom<int32> = Phantom<int32>.default();
/// @type.symbol symbol=marker source=marker type=Phantom<int32>
/// @resolution.pattern source=marker kind=binding target=marker
/// @generic.instance id=Phantom<int32> template=Phantom arguments=(int32)
/// @resolution.name source=Phantom target=Phantom
/// @type.node source=Phantom type=Phantom
/// @type.node source=Phantom<int32> type=Phantom<int32>
/// @type.node source=Phantom<int32>.default type=() => Phantom<int32>
/// @type.node source=Phantom<int32>.default() type=Phantom<int32>
/// @resolution.name source=Phantom target=Phantom
/// @resolution.name source=Phantom<int32> target=Phantom
/// @resolution.member source=Phantom<int32>.default receiver=Phantom<int32> type=() => Phantom<int32> kind=symbol target_receiver=Phantom<int32> target=default
/// @resolution.call source=Phantom<int32>.default() parameters=() return=Phantom<int32> kind=symbol target=default instance=Phantom<int32>.<extension#1>.default
/// @generic.instantiation id=default<int32> template=default arguments=(int32)
/// @generic.instance id=default<int32> template=default arguments=(int32)
/// @generic.instance id=new<int32> template=new arguments=(int32)
/// @generic.instance id=newPhantom<int32> template=newPhantom arguments=(int32)
"#,
    );
}
