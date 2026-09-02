use crate::tests::{DirRows, TestSession};

/// An exclusive borrow of an array binding reaches the array's freeing methods.
#[test]
fn test_push_through_an_exclusive_borrow_of_an_array_binding() {
    let session = TestSession::single(
        r#"
function grow(): void {
    let x = [1, 2];
    let x1 = &exclusive x;
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
    let x1: &'frame exclusive int64[] = &exclusive x;
    x1.push<int64>(3);
}

=== dir ===
function grow(): void {
/// @type.symbol symbol=grow type=() => void

    let x = [1, 2];
    /// @type.symbol symbol=grow.x source=x type=int64[]
    /// @resolution.pattern source=x kind=binding target=grow.x
    /// @generic.instance id=Array<int64> template=Array arguments=(int64)
    /// @generic.instance id=MaybeUninit<int64> template=MaybeUninit arguments=(int64)
    /// @generic.instance id=new<MaybeUninit<int64>> template=new arguments=(MaybeUninit<int64>)
    /// @resolution.call source=[1, 2] parameters=(&arrayFromSlice.'a readonly Slice<arrayFromSlice.T>) arguments=(rest(1, 2) as int64) return=int64[] kind=symbol target=arrayFromSlice instance=arrayFromSlice<int64>

    let x1 = &exclusive x;
    /// @type.symbol symbol=grow.x1 source=x1 type=&'frame exclusive int64[]
    /// @resolution.pattern source=x1 kind=binding target=grow.x1
    /// @resolution.name source=x target=grow.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=grow.x

    x1.push(3);
    /// @resolution.name source=x1 target=grow.x1
    /// @resolution.member source=x1.push receiver=&'frame exclusive int64[] type=<push.'a>(this: &push.'a exclusive int64[], ...int64[]) => isize kind=symbol target_receiver=&'frame exclusive int64[] target=push
    /// @resolution.call source=x1.push(3) parameters=(int64[]) arguments=(rest(3) pack=arrayFromSlice as int64) return=isize kind=symbol target=push receiver=&'frame exclusive int64[] instance=Array<int64>.<extension#5>.push
    /// @resolution.place source=x1 placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x1 root=grow.x1
    /// @generic.instantiation id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
    /// @generic.instantiation id=push<int64> template=push arguments=(int64)
    /// @generic.instance id=arrayFromSlice<int64> template=arrayFromSlice arguments=(int64)
    /// @generic.instance id=push<int64> template=push arguments=(int64)

}
"#,
    );
}
