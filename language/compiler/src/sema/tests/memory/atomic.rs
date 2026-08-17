use crate::tests::{DirRows, TestSession};

#[test]
fn test_accept_atomic_safe_storage_types() {
    let session = TestSession::single(
        r#"
import { Atomic, AtomicSafe } from "destack:sync";

declare const ready: Atomic<boolean>;
declare const count: Atomic<uint32>;
declare const index: Atomic<usize>;
declare const ratio: Atomic<float64>;

function read<T: AtomicSafe>(value: &readonly Atomic<T>): T {
    return value.load();
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Atomic, AtomicSafe } from "destack:sync";

declare const ready: Atomic<boolean>;
declare const count: Atomic<uint32>;
declare const index: Atomic<usize>;
declare const ratio: Atomic<float64>;

function read<T: AtomicSafe, 'a>(value: &'a readonly Atomic<T>): T {
    return value.load<T>();
}

=== dir ===
import { Atomic, AtomicSafe } from "destack:sync";

declare const ready: Atomic<boolean>;
/// @type.symbol symbol=ready source=ready type=sync.atomic.Atomic<boolean>
/// @resolution.pattern source=ready kind=binding target=ready
/// @generic.instance id=sync.atomic.Atomic<boolean> template=sync.atomic.Atomic arguments=(boolean)
/// @resolution.name source=Atomic target=sync.atomic.Atomic

declare const count: Atomic<uint32>;
/// @type.symbol symbol=count source=count type=sync.atomic.Atomic<uint32>
/// @resolution.pattern source=count kind=binding target=count
/// @generic.instance id=sync.atomic.Atomic<uint32> template=sync.atomic.Atomic arguments=(uint32)
/// @resolution.name source=Atomic target=sync.atomic.Atomic

declare const index: Atomic<usize>;
/// @type.symbol symbol=index source=index type=sync.atomic.Atomic<usize>
/// @resolution.pattern source=index kind=binding target=index
/// @generic.instance id=sync.atomic.Atomic<usize> template=sync.atomic.Atomic arguments=(usize)
/// @resolution.name source=Atomic target=sync.atomic.Atomic

declare const ratio: Atomic<float64>;
/// @type.symbol symbol=ratio source=ratio type=sync.atomic.Atomic<float64>
/// @resolution.pattern source=ratio kind=binding target=ratio
/// @generic.instance id=sync.atomic.Atomic<float64> template=sync.atomic.Atomic arguments=(float64)
/// @resolution.name source=Atomic target=sync.atomic.Atomic

function read<T: AtomicSafe>(value: &readonly Atomic<T>): T {
/// @generic.template symbol=read parameters=(T: sync.atomic.AtomicSafe, 'a)
/// @type.symbol symbol=read type=<T: sync.atomic.AtomicSafe, read.'a>(&read.'a readonly sync.atomic.Atomic<T>) => T
/// @type.symbol symbol=read.T source="T: AtomicSafe" type=T
/// @resolution.name source=AtomicSafe target=sync.atomic.AtomicSafe
/// @type.symbol symbol=read.value source="value: &readonly Atomic<T>" type=&read.'a readonly sync.atomic.Atomic<T>
/// @resolution.name source=Atomic target=sync.atomic.Atomic
/// @resolution.name source=T target=read.T
/// @resolution.name source=T target=read.T

    return value.load();
    /// @resolution.name source=value target=read.value
    /// @resolution.member source=value.load receiver=&read.'a readonly sync.atomic.Atomic<T> type=<sync.atomic.load.'a>(this: &sync.atomic.load.'a readonly &read.'a readonly sync.atomic.Atomic<T>, sync.atomic.MemoryOrdering?) => T kind=symbol target_receiver=&read.'a readonly sync.atomic.Atomic<T> target=sync.atomic.load
    /// @resolution.call source=value.load() parameters=(sync.atomic.MemoryOrdering) arguments=(omitted as sync.atomic.MemoryOrdering) return=T kind=symbol target=sync.atomic.load receiver=&read.'a readonly sync.atomic.Atomic<T> adjustments=(&read.'a readonly sync.atomic.Atomic<T> => direct -> sync.atomic.Atomic<T>, borrow(&read.'a readonly sync.atomic.Atomic<T>)) instance=sync.atomic.Atomic<T>.<extension#1>.load
    /// @resolution.place source=value placement="local" lifetime=read.'a access="readonly"
    /// @resolution.access source=value root=read.value
    /// @generic.instantiation id=sync.atomic.load<T> template=sync.atomic.load arguments=(T) owner=read

}
"#,
    );
}

#[test]
fn test_reject_unsupported_atomic_storage_type() {
    let session = TestSession::single(
        r#"
import { Atomic } from "destack:sync";

declare const wide: Atomic<uint128>;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Atomic } from "destack:sync";

declare const wide: Atomic<uint128>;

=== dir ===
import { Atomic } from "destack:sync";

declare const wide: Atomic<uint128>;
/// @type.symbol symbol=wide source=wide type=sync.atomic.Atomic<uint128>
/// @resolution.pattern source=wide kind=binding target=wide
/// @resolution.name source=Atomic target=sync.atomic.Atomic
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type 'uint128' does not satisfy 'AtomicSafe'"
/// @diagnostic.label line=4 column=28 span="uint128" line_source="declare const wide: Atomic<uint128>;"
/// @diagnostic.related file="atomic.ds" line=96 column=23 span="T" line_source="export newtype Atomic<T: AtomicSafe> = intrinsic;" message="required by this bound on 'T'"
"#,
    );
}

#[test]
fn test_reject_unsafe_atomic_safe_implementation_for_unsupported_storage() {
    let session = TestSession::single(
        r#"
import { AtomicSafe } from "destack:sync";

struct Word {
    bits: uint64;
}

@unsafe
extension of Word implements AtomicSafe {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_definitions().with_decorators(),
        r#"
=== annotated ===
import { AtomicSafe } from "destack:sync";

struct Word {
    bits: uint64;
}

@unsafe
extension of Word implements AtomicSafe {}

=== dir ===
import { AtomicSafe } from "destack:sync";

struct Word {
/// @type.symbol symbol=Word type=Word
/// @definition.struct symbol=Word
/// @definition.field symbol=Word.bits source="bits: uint64" key=bits type=uint64

    bits: uint64;
    /// @type.symbol symbol=Word.bits source="bits: uint64" type=uint64

}

@unsafe
/// @decorator.node source=@unsafe owner="extension of Word implements AtomicSafe {}" expression=unsafe target=decorator.unsafe type=unsafe kind=newtype parameters=() newtype=decorator.taint.unsafe backing=() value=unsafe()
/// @resolution.name source=unsafe target=decorator.taint.unsafe

extension of Word implements AtomicSafe {}
/// @definition.extension symbol=<module>#2 source="extension of Word implements AtomicSafe {}" form=local target=Word
/// @definition.implements symbol=<module>#2 source=AtomicSafe target=sync.atomic.AtomicSafe
/// @resolution.name source=Word target=Word
/// @resolution.name source=AtomicSafe target=sync.atomic.AtomicSafe
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Word' does not implement interface 'AtomicSafe'"
/// @diagnostic.label line=9 column=30 span="AtomicSafe" line_source="extension of Word implements AtomicSafe {}"
"#,
    );
}
