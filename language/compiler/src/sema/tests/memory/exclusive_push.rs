use crate::tests::{DirRows, TestSession};

/// A mutable borrow of an array binding reaches the array's push.
#[test]
fn test_push_through_a_mutable_borrow_of_an_array_binding() {
    let session = TestSession::single(
        r#"
function grow(): void {
    let x = [1, 2];
    let x1 = &x;
    x1.push(3);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function grow(): void {
    let x: int64[] = [1, 2];
    let x1: &'frame int64[] = &x;
    x1.push<int64, "frame">(3);
}

=== dir ===
function grow(): void {
/// @type.symbol symbol=grow type=() => void

    let x = [1, 2];
    /// @type.symbol symbol=grow.x source=x type=int64[]
    /// @resolution.pattern source=x kind=binding target=grow.x
    /// @generic.instance id=Array<int64> template=Array arguments=(int64)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int64>> template=sliceAssumeInit arguments=(MaybeUninit<int64>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int64>> template=sliceUninit arguments=(MaybeUninit<int64>)
    /// @resolution.call source=[1, 2] parameters=(^Slice<int64>) arguments=(rest(provided(1) as int64, provided(2) as int64) as int64) return=int64[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int64>
    /// @generic.instantiation id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)
    /// @generic.instance id=arrayFromOwnedSlice<int64> template=arrayFromOwnedSlice arguments=(int64)

    let x1 = &x;
    /// @type.symbol symbol=grow.x1 source=x1 type=&'frame int64[]
    /// @resolution.pattern source=x1 kind=binding target=grow.x1
    /// @resolution.name source=x target=grow.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=grow.x

    x1.push(3);
    /// @resolution.name source=x1 target=grow.x1
    /// @resolution.member source=x1.push receiver=&'frame int64[] type=<push.'a>(this: &push.'a int64[], ...int64[]) => isize kind=symbol target_receiver=&'frame int64[] target=push
    /// @resolution.call source=x1.push(3) parameters=(int64[]) arguments=(rest(provided(3) as int64) pack=arrayFromOwnedSlice as int64) return=isize regions=("frame" & "local") kind=symbol target=push receiver=&'frame int64[] instance="Array<int64>.<extension#6>.push<\"frame\" & \"local\">"
    /// @resolution.place source=x1 placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=x1 root=grow.x1
    /// @generic.instantiation id="push<int64, \"frame\" & \"local\">" template=push arguments=(int64, "frame" & "local")
    /// @generic.instantiation id=push<int64> template=push arguments=(int64)
    /// @generic.instance id="push<int64, \"bound0\" & \"local\">" template=push arguments=(int64, "bound0" & "local")

}
"#,
    );
}
