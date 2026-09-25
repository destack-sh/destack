use crate::tests::{DirRows, TestSession};

#[test]
fn test_accept_atomic_safe_storage_types() {
    let session = TestSession::single(
        r#"
import { Atomic, AtomicSafe, MemoryOrdering } from "tspp:sync";

declare const ready: Atomic<boolean>;
declare const count: Atomic<uint32>;
declare const index: Atomic<usize>;
declare const ratio: Atomic<float64>;

function read<T: AtomicSafe>(value: &readonly Atomic<T>): T {
    return value.load(MemoryOrdering.SequentiallyConsistent);
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Atomic, AtomicSafe, MemoryOrdering } from "tspp:sync";

declare const ready: Atomic<boolean>;
declare const count: Atomic<uint32>;
declare const index: Atomic<usize>;
declare const ratio: Atomic<float64>;

function read<T: AtomicSafe, 'a>(value: &'a readonly Atomic<T>): T {
    return value.load(MemoryOrdering.SequentiallyConsistent);
}

=== dir ===
import { Atomic, AtomicSafe, MemoryOrdering } from "tspp:sync";

declare const ready: Atomic<boolean>;
/// @type.symbol symbol=ready source=ready type=Atomic<boolean>
/// @resolution.pattern source=ready kind=binding target=ready
/// @generic.instance id=Atomic<boolean> template=Atomic arguments=(boolean)
/// @resolution.name source=Atomic target=Atomic

declare const count: Atomic<uint32>;
/// @type.symbol symbol=count source=count type=Atomic<uint32>
/// @resolution.pattern source=count kind=binding target=count
/// @generic.instance id=Atomic<uint32> template=Atomic arguments=(uint32)
/// @resolution.name source=Atomic target=Atomic

declare const index: Atomic<usize>;
/// @type.symbol symbol=index source=index type=Atomic<usize>
/// @resolution.pattern source=index kind=binding target=index
/// @generic.instance id=Atomic<usize> template=Atomic arguments=(usize)
/// @resolution.name source=Atomic target=Atomic

declare const ratio: Atomic<float64>;
/// @type.symbol symbol=ratio source=ratio type=Atomic<float64>
/// @resolution.pattern source=ratio kind=binding target=ratio
/// @generic.instance id=Atomic<float64> template=Atomic arguments=(float64)
/// @resolution.name source=Atomic target=Atomic

function read<T: AtomicSafe>(value: &readonly Atomic<T>): T {
/// @generic.template symbol=read parameters=(T: AtomicSafe, 'a)
/// @type.symbol symbol=read type=<T: AtomicSafe, read.'a>(&read.'a readonly Atomic<T>) => T
/// @generic.instance id=Atomic<T> template=Atomic arguments=(T)
/// @type.symbol symbol=read.T source="T: AtomicSafe" type=T
/// @resolution.name source=AtomicSafe target=AtomicSafe
/// @type.symbol symbol=read.value source="value: &readonly Atomic<T>" type=&read.'a readonly Atomic<T>
/// @resolution.name source=Atomic target=Atomic
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    return value.load(MemoryOrdering.SequentiallyConsistent);
    /// @resolution.name source=value target=read.value
    /// @resolution.member source=value.load receiver=&read.'a readonly Atomic<T> type=<const load.Order: MemoryOrdering.Relaxed | MemoryOrdering.Acquire | MemoryOrdering.SequentiallyConsistent = MemoryOrdering.SequentiallyConsistent, load.'a>(this: &load.'a readonly Atomic<T>, load.Order | undefined?) => T kind=symbol target_receiver=&read.'a readonly Atomic<T> target=load
    /// @resolution.call source=value.load(MemoryOrdering.SequentiallyConsistent) parameters=(MemoryOrdering.SequentiallyConsistent | undefined) arguments=(provided(MemoryOrdering.SequentiallyConsistent) as MemoryOrdering.SequentiallyConsistent | undefined) return=T regions=(read.'a) kind=symbol target=load receiver=&read.'a readonly Atomic<T> instance="Atomic<T>.<extension#2>.load<MemoryOrdering.SequentiallyConsistent, read.'a>"
    /// @resolution.place source=value placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=value root=read.value
    /// @generic.instantiation id="load<T, MemoryOrdering.SequentiallyConsistent, read.'a>" template=load arguments=(T, MemoryOrdering.SequentiallyConsistent, read.'a) owner=read
    /// @generic.instantiation id=load<T> template=load arguments=(T) owner=read
    /// @generic.instance id="get#1<T, read.'a>" template=get#1 arguments=(T, read.'a)
    /// @generic.instance id="load<T, MemoryOrdering.SequentiallyConsistent, read.'a>" template=load arguments=(T, MemoryOrdering.SequentiallyConsistent, read.'a)
    /// @generic.instance id=atomicLoad<T> template=atomicLoad arguments=(T)
    /// @resolution.name source=MemoryOrdering target=MemoryOrdering
    /// @resolution.member source=MemoryOrdering.SequentiallyConsistent receiver=MemoryOrdering type=MemoryOrdering.SequentiallyConsistent kind=symbol target_receiver=MemoryOrdering target=MemoryOrdering.SequentiallyConsistent

}
"#,
    );
}

#[test]
fn test_reject_unsupported_atomic_storage_type() {
    let session = TestSession::single(
        r#"
import { Atomic } from "tspp:sync";

declare const wide: Atomic<uint128>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Atomic } from "tspp:sync";

declare const wide: Atomic<uint128>;

=== dir ===
import { Atomic } from "tspp:sync";

declare const wide: Atomic<uint128>;
/// @type.symbol symbol=wide source=wide type=Atomic<uint128>
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=Atomic target=Atomic
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'uint128' does not satisfy 'AtomicSafe'"
/// @diagnostic.label line=4 column=28 span="uint128" line_source="declare const wide: Atomic<uint128>;"
/// @diagnostic.related file="atomic.tspp" line=98 column=22 span="T" line_source="export struct Atomic<T: AtomicSafe> {" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_reject_unsafe_atomic_safe_implementation_for_unsupported_storage() {
    let session = TestSession::single(
        r#"
import { AtomicSafe } from "tspp:sync";

struct Word {
    bits: uint64;
}

@unsafe
extension of Word implements AtomicSafe {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_definitions().with_decorators(),
        r#"
=== annotated ===
import { AtomicSafe } from "tspp:sync";

struct Word {
    bits: uint64;
}

@unsafe
extension of Word implements AtomicSafe {}

=== dir ===
import { AtomicSafe } from "tspp:sync";

struct Word {
/// @type.symbol symbol=Word type=Word
/// @definition.struct symbol=Word
/// @definition.field symbol=Word.bits source="bits: uint64" key=bits type=uint64

    bits: uint64;
    /// @type.symbol symbol=Word.bits source="bits: uint64" type=uint64

}

@unsafe
/// @decorator.node source=@unsafe owner="extension of Word implements AtomicSafe {}" expression=unsafe target=decorator.unsafe type=unsafe kind=newtype parameters=() newtype=unsafe backing=() value=unsafe()
/// @resolution.name source=unsafe target=unsafe

extension of Word implements AtomicSafe {}
/// @definition.extension symbol=<module>#2 source="extension of Word implements AtomicSafe {}" form=local target=Word
/// @definition.implements symbol=<module>#2 source=AtomicSafe target=AtomicSafe
/// @resolution.name source=Word target=Word
/// @resolution.name source=AtomicSafe target=AtomicSafe
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Word' does not implement interface 'AtomicSafe'"
/// @diagnostic.label line=9 column=30 span="AtomicSafe" line_source="extension of Word implements AtomicSafe {}"
"#,
    );
}
