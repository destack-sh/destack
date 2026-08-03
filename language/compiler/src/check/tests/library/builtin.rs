use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::{DirRows, TestSession};

/// Check every builtin library module.
#[test]
fn test_check_library() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let target = TargetId::new(package.package_id(), "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .unwrap_or_else(|error| panic!("builtin library target profile should resolve:\n{error}"))
        .id();

    // check every builtin module through the normal artifact path
    let keys = package
        .module_ids()
        .map(|module| ArtifactKey::dir_checked(module, profile))
        .collect::<Vec<_>>();
    let result = session.require_all(keys.iter().copied());
    let diagnostics = session.render_terminal_diagnostics_for(&keys);
    if !diagnostics.is_empty() {
        panic!("\n{diagnostics}");
    }

    if let Err(error) = result {
        panic!("{error}");
    }
}

/// Resolve prelude extension members on every receiver family.
///
/// Each binding pins one cell: blanket members on literal and variable
/// scalars, and rooted members on managed and borrowed collections.
#[test]
fn test_resolve_prelude_extension_members() {
    let session = TestSession::single(
        r#"
declare const half: float64;
declare const count: int32;
declare const name: string;
declare const items: Array<int32>;
declare const view: &readonly Array<int32>;

const root = (3.5).sqrt();
const nan = half.isNaN();
const scaled = half.abs().min(1.0);
const power = count.isPowerOfTwo();
const letters = name.length;
const total = items.length;
const seen = view.length;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare const half: float64;
declare const count: int32;
declare const name: string;
declare const items: Array<int32>;
declare const view: &'static readonly Array<int32>;

const root: float64 = 3.5.sqrt<float64>();
const nan: boolean = half.isNaN<float64>();
const scaled: float64 = half.abs<float64>().min<float64>(1.0);
const power: boolean = count.isPowerOfTwo<int32>();
const letters: usize = name.length;
const total: usize = items.length;
const seen: usize = view.length;

=== checked ===
declare const half: float64;
/// @type.symbol symbol=half source=half type=float64
/// @resolution.pattern source=half kind=binding target=half

declare const count: int32;
/// @type.symbol symbol=count source=count type=int32
/// @resolution.pattern source=count kind=binding target=count

declare const name: string;
/// @type.symbol symbol=name source=name type=string
/// @resolution.pattern source=name kind=binding target=name

declare const items: Array<int32>;
/// @type.symbol symbol=items source=items type=Array<int32>
/// @resolution.pattern source=items kind=binding target=items
/// @resolution.name source=Array target=collections.array.Array

declare const view: &readonly Array<int32>;
/// @type.symbol symbol=view source=view type=&'static readonly Array<int32>
/// @resolution.pattern source=view kind=binding target=view
/// @resolution.name source=Array target=collections.array.Array

const root = (3.5).sqrt();
/// @type.symbol symbol=root source=root type=float64
/// @resolution.pattern source=root kind=binding target=root
/// @resolution.member source=(3.5).sqrt receiver=3.5 type=(this: 3.5) => float64 kind=symbol target_receiver=3.5 target=math.float.sqrt#1
/// @resolution.call source=(3.5).sqrt() parameters=() return=float64 kind=symbol target=math.float.sqrt#1 receiver=3.5 instance=float64.<extension#1>.sqrt#1
/// @generic.instance source=(3.5).sqrt() id=float64.<extension#1>.sqrt#1

const nan = half.isNaN();
/// @type.symbol symbol=nan source=nan type=boolean
/// @resolution.pattern source=nan kind=binding target=nan
/// @resolution.name source=half target=half
/// @resolution.member source=half.isNaN receiver=float64 type=(this: float64) => boolean kind=symbol target_receiver=float64 target=math.float.isNaN
/// @resolution.call source=half.isNaN() parameters=() return=boolean kind=symbol target=math.float.isNaN receiver=float64 instance=float64.<extension#1>.isNaN
/// @resolution.place source=half placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=half root=half
/// @generic.instance source=half.isNaN() id=float64.<extension#1>.isNaN

const scaled = half.abs().min(1.0);
/// @type.symbol symbol=scaled source=scaled type=float64
/// @resolution.pattern source=scaled kind=binding target=scaled
/// @resolution.name source=half target=half
/// @resolution.member source=half.abs receiver=float64 type=(this: float64) => float64 kind=symbol target_receiver=float64 target=math.float.abs#1
/// @resolution.member source=half.abs().min receiver=float64 type=(this: float64, float64) => float64 kind=symbol target_receiver=float64 target=math.float.min#1
/// @resolution.call source=half.abs() parameters=() return=float64 kind=symbol target=math.float.abs#1 receiver=float64 instance=float64.<extension#1>.abs#1
/// @resolution.call source=half.abs().min(1.0) parameters=(float64) arguments=(provided(1.0) as float64) return=float64 kind=symbol target=math.float.min#1 receiver=float64 instance=float64.<extension#1>.min#1
/// @resolution.place source=half placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=half root=half
/// @generic.instance source=half.abs() id=float64.<extension#1>.abs#1
/// @generic.instance source=half.abs().min(1.0) id=float64.<extension#1>.min#1

const power = count.isPowerOfTwo();
/// @type.symbol symbol=power source=power type=boolean
/// @resolution.pattern source=power kind=binding target=power
/// @resolution.name source=count target=count
/// @resolution.member source=count.isPowerOfTwo receiver=int32 type=(this: int32) => boolean kind=symbol target_receiver=int32 target=math.integer.isPowerOfTwo
/// @resolution.call source=count.isPowerOfTwo() parameters=() return=boolean kind=symbol target=math.integer.isPowerOfTwo receiver=int32 instance=int32.<extension#1>.isPowerOfTwo
/// @resolution.place source=count placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=count root=count
/// @generic.instance source=count.isPowerOfTwo() id=int32.<extension#1>.isPowerOfTwo

const letters = name.length;
/// @type.symbol symbol=letters source=letters type=usize
/// @resolution.pattern source=letters kind=binding target=letters
/// @resolution.name source=name target=name
/// @resolution.member source=name.length receiver=string type=usize kind=call target="string.string.length(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=name placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=name root=name

const total = items.length;
/// @type.symbol symbol=total source=total type=usize
/// @resolution.pattern source=total kind=binding target=total
/// @resolution.name source=items target=items
/// @resolution.member source=items.length receiver=Array<int32> type=usize kind=call target="collections.array.length#2(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=items placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=items root=items
/// @generic.instance source=items.length id=Array<int32>.<extension#2>.length#2

const seen = view.length;
/// @type.symbol symbol=seen source=seen type=usize
/// @resolution.pattern source=seen kind=binding target=seen
/// @resolution.name source=view target=view
/// @resolution.member source=view.length receiver=&'static readonly Array<int32> type=usize kind=call target="collections.array.length#2(parameters=(), arguments=(), return=usize)"
/// @resolution.place source=view placement="local" lifetime="static" access="readonly"
/// @resolution.access source=view root=view
/// @generic.instance source=view.length id=Array<int32>.<extension#2>.length#2

/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=Array<int32>.<extension#2>.length#2 template=collections.array.length#2 arguments=(int32)
/// @generic.instance id=float64.<extension#1>.abs#1 template=math.float.abs#1 arguments=(float64)
/// @generic.instance id=float64.<extension#1>.isNaN template=math.float.isNaN arguments=(float64)
/// @generic.instance id=float64.<extension#1>.min#1 template=math.float.min#1 arguments=(float64)
/// @generic.instance id=float64.<extension#1>.sqrt#1 template=math.float.sqrt#1 arguments=(float64)
/// @generic.instance id=int32.<extension#1>.isPowerOfTwo template=math.integer.isPowerOfTwo arguments=(int32)
"#,
    );
}
