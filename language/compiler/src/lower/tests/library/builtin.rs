use destack_artifact::ArtifactKey;
use destack_source::TargetId;

use crate::tests::TestSession;
use crate::tests::snapshot::assert_snapshot;

/// Every builtin module lowers through the artifact path.
#[test]
fn test_lower_library() {
    let session = TestSession::builder().cold().build();
    let repository = session.repository();
    let package = repository.embedded_builtin();
    let package_id = package.package_id();
    let target = TargetId::new(package_id, "default");
    let profile = repository
        .profile_for_target(session.revision(), target)
        .expect("builtin library target profile should resolve")
        .id();

    // build every module concurrently before the ordered per-module readout
    let keys: Vec<_> = package
        .files()
        .iter()
        .map(|file| ArtifactKey::mir_lowered(file.module_id(), profile, target))
        .collect();
    let _ = session.require_all(keys.iter().copied());

    // report each module's outcome in declaration order
    let mut report = String::new();
    for file in package.files() {
        let key = ArtifactKey::mir_lowered(file.module_id(), profile, target);
        let Err(error) = session.require_all([key]) else {
            report.push_str(&format!("ok   {}\n", file.path));

            continue;
        };

        // keep the rendered diagnostics, or the trailing error text when none render
        let rendered = session.render_terminal_diagnostics_for(&[key]);
        let rendered = strip_terminal_styles(&rendered);
        let mut lines = rendered
            .lines()
            .filter(|line| line.contains("error["))
            .peekable();
        if lines.peek().is_none() {
            let message = error.to_string();
            let message = match message.rsplit_once("}: ") {
                Some((_, tail)) => tail.to_string(),
                None => message,
            };
            let message = message.lines().next().unwrap_or_default();
            report.push_str(&format!("FAIL {} :: {message}\n", file.path));
        }
        let mut reported = Vec::new();
        for line in lines {
            let line = line.trim();
            if reported.iter().any(|known: &String| known == line) {
                continue;
            }

            reported.push(line.to_string());
            report.push_str(&format!("FAIL {} :: {line}\n", file.path));
        }
    }

    assert_snapshot(
        report,
        r"
FAIL assert/assert.ds :: error[unsupported-lower-construct]: unsupported construct: 'Is' expressions
ok   assert/index.ds
ok   async/abort.ds
ok   async/awaitable.ds
ok   async/fiber.ds
ok   async/generator.ds
ok   async/index.ds
ok   async/iterator.ds
ok   async/mutex.ds
ok   async/notify.ds
ok   async/once.ds
ok   async/poll.ds
FAIL async/promise.ds :: a union conversion selecting an absent target member
FAIL async/reader.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   async/result.ds
ok   async/rwlock.ds
FAIL async/seek.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
FAIL async/semaphore.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   async/task.ds
FAIL async/writer.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
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
FAIL channel/watch.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   collections/array.ds
ok   collections/concurrent-map.ds
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
ok   collections/slice.ds
ok   collections/small-array.ds
ok   collections/sorted-map.ds
ok   collections/sorted-set.ds
FAIL console/console.ds :: error[unsupported-lower-construct]: unsupported construct: a spread argument
ok   console/index.ds
ok   context/context.ds
ok   context/index.ds
ok   context/variable.ds
ok   convert/borrow.ds
ok   convert/from.ds
ok   convert/index.ds
ok   convert/into.ds
ok   convert/reference.ds
ok   debug/debug.ds
FAIL debug/format.ds :: error[unsupported-lower-construct]: unsupported construct: a spread argument
ok   debug/index.ds
ok   decorator/capture.ds
FAIL decorator/derive.ds :: error[unsupported-lower-construct]: unsupported construct: the 'decorator.derive' intrinsic representation
FAIL decorator/diagnostic.ds :: error[unsupported-lower-construct]: unsupported construct: a layout for this type
ok   decorator/index.ds
ok   decorator/intrinsic.ds
FAIL decorator/representation.ds :: an unreduced 'Operation' type
ok   decorator/restriction.ds
ok   decorator/safety.ds
ok   decorator/stability.ds
ok   decorator/system.ds
ok   error/debug.ds
ok   error/error.ds
FAIL error/host.ds :: a coalesce joining reference and value representations
ok   error/index.ds
FAIL error/io.ds :: a coalesce joining reference and value representations
ok   error/panic.ds
ok   error/report.ds
ok   error/result.ds
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
FAIL intl/error.ds :: error[unsupported-lower-construct]: unsupported construct: a member read on a union receiver
ok   intl/format.ds
ok   intl/index.ds
ok   intl/list-format.ds
ok   intl/locale.ds
ok   intl/number-format.ds
ok   intl/plural-rules.ds
ok   intl/relative-time-format.ds
ok   intl/segmenter.ds
ok   iter/index.ds
ok   iter/iterator.ds
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
ok   math/linear.ds
ok   math/math.ds
ok   math/matrix.ds
ok   math/number.ds
ok   math/numeric.ds
ok   math/quaternion.ds
ok   math/vector.ds
ok   math/wrapping.ds
ok   memory/access.ds
ok   memory/arc/arc.ds
ok   memory/arc/index.ds
ok   memory/arc/weak.ds
ok   memory/arena/arena.ds
FAIL memory/arena/bump.ds :: a partially applied nominal argument list
ok   memory/arena/index.ds
ok   memory/binding/advise.ds
ok   memory/binding/index.ds
ok   memory/binding/layout.ds
ok   memory/binding/lock.ds
ok   memory/binding/map.ds
ok   memory/binding/memory.ds
ok   memory/binding/virtual.ds
ok   memory/borrow.ds
ok   memory/box.ds
ok   memory/capability.ds
ok   memory/cell/cell.ds
ok   memory/cell/index.ds
ok   memory/cell/refcell.ds
ok   memory/cow/cow.ds
ok   memory/cow/index.ds
ok   memory/dispose.ds
ok   memory/drop.ds
ok   memory/dynamic.ds
ok   memory/error.ds
ok   memory/index.ds
ok   memory/init.ds
ok   memory/lifetime.ds
ok   memory/managed.ds
ok   memory/owned.ds
ok   memory/phantom.ds
ok   memory/pin.ds
ok   memory/place.ds
ok   memory/raw.ds
ok   memory/rc/index.ds
ok   memory/rc/rc.ds
ok   memory/rc/weak.ds
FAIL memory/region.ds :: error[unsupported-lower-construct]: unsupported construct: the 'memory.Region' intrinsic representation
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
FAIL net/net.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
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
FAIL profile/instrument.ds :: error[unsupported-lower-construct]: unsupported construct: the 'profile.Counter' intrinsic representation
ok   random/binding/entropy.ds
ok   random/binding/index.ds
ok   random/binding/random.ds
ok   random/index.ds
FAIL random/random.ds :: an adapted value outside its declared representation
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
FAIL runtime/world.ds :: an adapted value outside its declared representation
ok   serde/index.ds
ok   serde/serde.ds
ok   signal/accessor.ds
ok   signal/action.ds
ok   signal/control.ds
ok   signal/derive.ds
ok   signal/effect.ds
ok   signal/index.ds
ok   signal/map.ds
ok   signal/memo.ds
ok   signal/optimistic.ds
ok   signal/options.ds
ok   signal/owner.ds
ok   signal/reaction.ds
ok   signal/setter.ds
ok   signal/signal.ds
ok   stream/index.ds
ok   stream/stream.ds
FAIL string/builder.ds :: error[unsupported-lower-construct]: unsupported construct: the 'char' type
FAIL string/character.ds :: error[unsupported-lower-construct]: unsupported construct: the 'char' type
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
FAIL telemetry/log.ds :: error[unsupported-lower-construct]: unsupported construct: 'Chain' expressions
ok   telemetry/metric.ds
ok   telemetry/record.ds
FAIL telemetry/trace.ds :: a non-union type in a union member read
ok   test/artifact.ds
FAIL test/body.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
FAIL test/case.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
FAIL test/context.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   test/expect.ds
FAIL test/fixture.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   test/hook.ds
ok   test/id.ds
ok   test/index.ds
ok   test/issue.ds
ok   test/options.ds
FAIL test/poll.ds :: error[unsupported-lower-construct]: unsupported construct: 'Match' statements
ok   test/replay.ds
ok   test/run.ds
ok   test/snapshot.ds
ok   test/suite.ds
ok   time/binding/clock.ds
ok   time/binding/index.ds
ok   time/binding/time.ds
ok   time/binding/timer.ds
ok   time/duration.ds
FAIL time/error.ds :: error[unsupported-lower-construct]: unsupported construct: a member read on a union receiver
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
ok   topology/edge.ds
ok   topology/entity.ds
ok   topology/index.ds
ok   topology/label.ds
FAIL topology/topology.ds :: an adapted value outside its declared representation
ok   tree/builder.ds
ok   tree/index.ds
ok   types/function.ds
ok   types/index.ds
ok   types/object.ds
ok   types/string.ds
ok   worker/index.ds
ok   worker/worker.ds
",
    );
}

/// Remove the terminal color codes from one rendered report.
fn strip_terminal_styles(rendered: &str) -> String {
    let mut stripped = String::with_capacity(rendered.len());
    let mut in_escape = false;
    for character in rendered.chars() {
        match (in_escape, character) {
            (false, '\u{1b}') => in_escape = true,
            (false, character) => stripped.push(character),
            (true, 'm') => in_escape = false,
            (true, _) => {}
        }
    }

    stripped
}
