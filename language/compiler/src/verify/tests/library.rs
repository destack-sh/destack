use destack_artifact::ArtifactKey;

use crate::tests::library_report;
use crate::tests::snapshot::assert_snapshot;

/// Every builtin module verifies through the artifact path.
#[test]
fn test_verify_library() {
    let report = library_report(ArtifactKey::mir_verified);

    assert_snapshot(
        report,
        r"
ok   assert/assert.ds
ok   assert/index.ds
ok   async/abort.ds
ok   async/awaitable.ds
ok   async/channel.ds
ok   async/fiber.ds
FAIL async/generator.ds :: error[unsupported-lower-construct]: unsupported construct: a must on a try implementor
ok   async/index.ds
ok   async/iterator.ds
ok   async/mutex.ds
ok   async/notify.ds
ok   async/once.ds
FAIL async/poll.ds :: error[unsupported-lower-construct]: unsupported construct: a match over an indirect scrutinee
FAIL async/promise.ds :: a union conversion requiring a missing source discriminant
ok   async/reader.ds
FAIL async/result.ds :: error[unsupported-lower-construct]: unsupported construct: a match over an indirect scrutinee
ok   async/rwlock.ds
ok   async/seek.ds
ok   async/semaphore.ds
ok   async/task.ds
ok   async/writer.ds
ok   binding/binding.ds
ok   binding/index.ds
ok   bytes/buffer.ds
ok   bytes/bytes.ds
ok   bytes/index.ds
ok   bytes/reader.ds
ok   channel/broadcast.ds
ok   channel/channel.ds
ok   channel/index.ds
ok   channel/mpmc.ds
ok   channel/mpsc.ds
ok   channel/oneshot.ds
ok   channel/watch.ds
ok   collections/array.ds
FAIL collections/concurrent-map.ds :: error[unsupported-lower-construct]: unsupported construct: a defaulted or spread intrinsic argument
ok   collections/concurrent-queue.ds
ok   collections/concurrent-set.ds
ok   collections/deque.ds
ok   collections/fixed-array.ds
ok   collections/heap.ds
ok   collections/index.ds
ok   collections/list.ds
ok   collections/map.ds
ok   collections/sequence.ds
ok   collections/set.ds
ok   collections/slab.ds
FAIL collections/slice.ds :: error[overwrite-without-exclusive]: cannot overwrite this storage without exclusive access
ok   collections/small-array.ds
ok   collections/sorted-map.ds
ok   collections/sorted-set.ds
ok   console/console.ds
ok   console/index.ds
FAIL context/context.ds :: error[unsupported-lower-construct]: unsupported construct: 'Try' statements
ok   context/index.ds
ok   context/variable.ds
ok   convert/borrow.ds
ok   convert/from.ds
ok   convert/index.ds
ok   convert/into.ds
ok   convert/reference.ds
ok   debug/debug.ds
ok   debug/format.ds
ok   debug/index.ds
ok   decorator/capture.ds
ok   decorator/derive.ds
ok   decorator/diagnostic.ds
ok   decorator/index.ds
ok   decorator/intrinsic.ds
ok   decorator/representation.ds
ok   decorator/restriction.ds
ok   decorator/safety.ds
ok   decorator/stability.ds
ok   decorator/system.ds
ok   error/debug.ds
ok   error/error.ds
ok   error/host.ds
ok   error/index.ds
ok   error/io.ds
ok   error/panic.ds
ok   error/report.ds
FAIL error/result.ds :: error[unsupported-lower-construct]: unsupported construct: a match over an indirect scrutinee
ok   error/stack.ds
ok   fs/binding/attribute.ds
ok   fs/binding/directory.ds
ok   fs/binding/entry.ds
ok   fs/binding/file.ds
ok   fs/binding/fs.ds
ok   fs/binding/index.ds
ok   fs/binding/mount.ds
ok   fs/binding/namespace.ds
ok   fs/binding/path.ds
ok   fs/binding/range.ds
ok   fs/binding/status.ds
ok   fs/binding/volume.ds
ok   fs/binding/watch.ds
ok   fs/binding/xattr.ds
ok   fs/fs.ds
ok   fs/index.ds
ok   fs/path.ds
ok   hint/hint.ds
ok   hint/index.ds
ok   index.ds
ok   intl/collator.ds
ok   intl/common.ds
ok   intl/date-time-format.ds
ok   intl/display-names.ds
ok   intl/duration-format.ds
ok   intl/error.ds
ok   intl/format.ds
ok   intl/index.ds
ok   intl/list-format.ds
ok   intl/locale.ds
ok   intl/number-format.ds
ok   intl/plural-rules.ds
ok   intl/relative-time-format.ds
ok   intl/segmenter.ds
ok   iter/index.ds
FAIL iter/iterator.ds :: error[unsupported-lower-construct]: unsupported construct: a generic type of 'Iterator' outside its template
ok   json/codec.ds
ok   json/error.ds
ok   json/index.ds
ok   json/json.ds
ok   json/value.ds
ok   math/arithmetic.ds
ok   math/bigint.ds
ok   math/cast.ds
ok   math/complex.ds
ok   math/float.ds
ok   math/identity.ds
ok   math/index.ds
ok   math/integer.ds
FAIL math/linear.ds :: an equality type outside a supported representation
ok   math/math.ds
FAIL math/matrix.ds :: an equality type outside a supported representation
ok   math/number.ds
ok   math/numeric.ds
ok   math/quaternion.ds
FAIL math/vector.ds :: error[unsupported-lower-construct]: unsupported construct: the 'math.Vector' intrinsic representation
FAIL math/wrapping.ds :: error[unsupported-lower-construct]: unsupported construct: the 'Parameter' type
ok   memory/access.ds
ok   memory/arc/arc.ds
ok   memory/arc/index.ds
ok   memory/arc/weak.ds
ok   memory/arena/arena.ds
ok   memory/arena/bump.ds
ok   memory/arena/index.ds
ok   memory/binding/advise.ds
ok   memory/binding/index.ds
ok   memory/binding/layout.ds
ok   memory/binding/lock.ds
ok   memory/binding/map.ds
ok   memory/binding/memory.ds
ok   memory/binding/virtual.ds
ok   memory/borrow.ds
FAIL memory/box.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.unique.leak' intrinsic
FAIL memory/box.ds :: error[unsupported-lower-construct]: unsupported construct: the 'Application' type
ok   memory/capability.ds
ok   memory/cell/cell.ds
ok   memory/cell/index.ds
ok   memory/cell/refcell.ds
ok   memory/cow/cow.ds
ok   memory/cow/index.ds
ok   memory/dispose.ds
FAIL memory/drop.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.ManuallyDrop' intrinsic representation
FAIL memory/dynamic.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.dynamic.type' intrinsic
FAIL memory/dynamic.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.dynamic.payload' intrinsic
ok   memory/error.ds
ok   memory/index.ds
ok   memory/init.ds
ok   memory/lifetime.ds
ok   memory/managed.ds
FAIL memory/owned.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.owned.intoManaged' intrinsic
FAIL memory/phantom.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.phantom.new' intrinsic
ok   memory/pin.ds
ok   memory/place.ds
ok   memory/raw.ds
ok   memory/rc/index.ds
ok   memory/rc/rc.ds
ok   memory/rc/weak.ds
ok   memory/region.ds
ok   memory/replace.ds
ok   memory/swap.ds
ok   memory/type.ds
ok   module/config.ds
ok   module/index.ds
ok   module/meta.ds
ok   net/binding/address.ds
ok   net/binding/connectivity.ds
ok   net/binding/index.ds
ok   net/binding/interface.ds
ok   net/binding/net.ds
ok   net/binding/options.ds
ok   net/binding/raw.ds
ok   net/binding/resolve.ds
ok   net/binding/route.ds
ok   net/binding/socket.ds
ok   net/binding/udp.ds
ok   net/index.ds
ok   net/net.ds
ok   ops/bitwise.ds
ok   ops/comparison.ds
ok   ops/dereference.ds
ok   ops/divide.ds
ok   ops/equality.ds
ok   ops/format.ds
ok   ops/hash.ds
ok   ops/index.ds
ok   ops/minus.ds
ok   ops/multiply.ds
ok   ops/negate.ds
ok   ops/plus.ds
ok   ops/power.ds
ok   ops/remainder.ds
ok   ops/shift.ds
ok   ops/subscript.ds
ok   ops/try.ds
ok   prelude.ds
ok   profile/index.ds
ok   profile/instrument.ds
ok   random/binding/entropy.ds
ok   random/binding/index.ds
ok   random/binding/random.ds
ok   random/index.ds
ok   random/random.ds
ok   range/bound.ds
ok   range/index.ds
ok   range/iterator.ds
ok   range/range.ds
ok   range/step.ds
ok   reflect/index.ds
ok   reflect/type.ds
ok   regexp/index.ds
ok   regexp/regexp.ds
ok   runtime/action.ds
ok   runtime/binding.ds
ok   runtime/control.ds
ok   runtime/entity.ds
ok   runtime/frame.ds
ok   runtime/heap.ds
ok   runtime/index.ds
ok   runtime/inspect.ds
ok   runtime/lineage.ds
ok   runtime/observation.ds
ok   runtime/policy.ds
ok   runtime/random.ds
ok   runtime/resource.ds
ok   runtime/snapshot.ds
ok   runtime/trace.ds
ok   runtime/world.ds
ok   serde/index.ds
ok   serde/serde.ds
ok   signal/accessor.ds
ok   signal/action.ds
ok   signal/control.ds
ok   signal/derive.ds
FAIL signal/effect.ds :: a union without its recorded canonical members
ok   signal/index.ds
FAIL signal/map.ds :: a union without its recorded canonical members
FAIL signal/memo.ds :: a union without its recorded canonical members
FAIL signal/optimistic.ds :: a union without its recorded canonical members
ok   signal/options.ds
ok   signal/owner.ds
ok   signal/reaction.ds
ok   signal/setter.ds
FAIL signal/signal.ds :: a union without its recorded canonical members
ok   stream/index.ds
ok   stream/stream.ds
ok   string/builder.ds
ok   string/character.ds
ok   string/cstring.ds
ok   string/index.ds
ok   string/os.ds
ok   string/slice.ds
ok   string/string.ds
ok   string/utf8.ds
ok   sync/atomic.ds
ok   sync/barrier.ds
ok   sync/condvar.ds
ok   sync/index.ds
ok   sync/mutex.ds
ok   sync/once.ds
ok   sync/reader.ds
ok   sync/rwlock.ds
ok   sync/seek.ds
ok   sync/semaphore.ds
ok   sync/writer.ds
ok   telemetry/binding/index.ds
ok   telemetry/binding/telemetry.ds
ok   telemetry/field.ds
ok   telemetry/index.ds
ok   telemetry/log.ds
ok   telemetry/metric.ds
ok   telemetry/record.ds
ok   telemetry/trace.ds
ok   test/artifact.ds
ok   test/body.ds
FAIL test/case.ds :: an unreduced value intersection
ok   test/context.ds
FAIL test/expect.ds :: an equality type outside a supported representation
ok   test/fixture.ds
ok   test/hook.ds
ok   test/id.ds
ok   test/index.ds
ok   test/issue.ds
ok   test/options.ds
ok   test/poll.ds
ok   test/replay.ds
ok   test/run.ds
ok   test/snapshot.ds
ok   test/suite.ds
ok   time/binding/clock.ds
ok   time/binding/index.ds
ok   time/binding/time.ds
ok   time/binding/timer.ds
ok   time/duration.ds
ok   time/error.ds
ok   time/index.ds
ok   time/instant.ds
ok   time/now.ds
ok   time/option.ds
ok   time/plain-date-time.ds
ok   time/plain-date.ds
ok   time/plain-month-day.ds
ok   time/plain-time.ds
ok   time/plain-year-month.ds
ok   time/unit.ds
ok   time/zoned-date-time.ds
ok   topology/binding/edge.ds
ok   topology/binding/entity.ds
ok   topology/binding/index.ds
ok   topology/binding/topology.ds
ok   topology/decorator.ds
FAIL topology/edge.ds :: error[unsupported-lower-construct]: unsupported construct: a 'symbol' member read
FAIL topology/entity.ds :: error[unsupported-lower-construct]: unsupported construct: a 'symbol' member read
ok   topology/index.ds
ok   topology/label.ds
ok   topology/topology.ds
ok   tree/builder.ds
ok   tree/index.ds
ok   types/boolean.ds
ok   types/function.ds
ok   types/index.ds
ok   types/object.ds
ok   types/string.ds
ok   worker/index.ds
ok   worker/worker.ds
",
    );
}
