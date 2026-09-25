use tspp_artifact::ArtifactKey;
use tspp_source::TargetId;

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
ok   assert/assert.tspp
ok   assert/index.tspp
ok   async/abort.tspp
ok   async/awaitable.tspp
ok   async/channel.tspp
ok   async/fiber.tspp
ok   async/generator.tspp
ok   async/index.tspp
ok   async/iterator.tspp
ok   async/mutex.tspp
ok   async/notify.tspp
ok   async/once.tspp
ok   async/poll.tspp
ok   async/promise.tspp
ok   async/reader.tspp
ok   async/result.tspp
ok   async/rwlock.tspp
ok   async/seek.tspp
ok   async/semaphore.tspp
ok   async/task.tspp
ok   async/writer.tspp
ok   binding/binding.tspp
ok   binding/index.tspp
ok   bytes/buffer.tspp
ok   bytes/bytes.tspp
ok   bytes/index.tspp
ok   bytes/reader.tspp
ok   channel/broadcast.tspp
ok   channel/channel.tspp
ok   channel/index.tspp
ok   channel/mpmc.tspp
ok   channel/mpsc.tspp
ok   channel/oneshot.tspp
ok   channel/watch.tspp
ok   collections/array.tspp
ok   collections/concurrent-map.tspp
ok   collections/concurrent-queue.tspp
ok   collections/concurrent-set.tspp
ok   collections/deque.tspp
ok   collections/fixed-array.tspp
ok   collections/heap.tspp
ok   collections/index.tspp
ok   collections/list.tspp
ok   collections/map.tspp
ok   collections/sequence.tspp
ok   collections/set.tspp
ok   collections/slab.tspp
ok   collections/slice.tspp
ok   collections/small-array.tspp
ok   collections/sorted-map.tspp
ok   collections/sorted-set.tspp
ok   console/console.tspp
ok   console/index.tspp
ok   context/context.tspp
ok   context/index.tspp
ok   context/variable.tspp
ok   convert/borrow.tspp
ok   convert/from.tspp
ok   convert/index.tspp
ok   convert/into.tspp
ok   convert/reference.tspp
ok   debug/debug.tspp
ok   debug/format.tspp
ok   debug/index.tspp
ok   decorator/capture.tspp
ok   decorator/derive.tspp
ok   decorator/diagnostic.tspp
ok   decorator/index.tspp
ok   decorator/intrinsic.tspp
ok   decorator/representation.tspp
ok   decorator/restriction.tspp
ok   decorator/safety.tspp
ok   decorator/stability.tspp
ok   decorator/system.tspp
ok   error/debug.tspp
ok   error/error.tspp
ok   error/host.tspp
ok   error/index.tspp
ok   error/io.tspp
ok   error/panic.tspp
ok   error/report.tspp
ok   error/result.tspp
ok   error/stack.tspp
ok   fs/binding/attribute.tspp
ok   fs/binding/directory.tspp
ok   fs/binding/entry.tspp
ok   fs/binding/file.tspp
ok   fs/binding/fs.tspp
ok   fs/binding/index.tspp
ok   fs/binding/mount.tspp
ok   fs/binding/namespace.tspp
ok   fs/binding/path.tspp
ok   fs/binding/range.tspp
ok   fs/binding/status.tspp
ok   fs/binding/volume.tspp
ok   fs/binding/watch.tspp
ok   fs/binding/xattr.tspp
ok   fs/fs.tspp
ok   fs/index.tspp
ok   fs/path.tspp
ok   hint/hint.tspp
ok   hint/index.tspp
ok   index.tspp
ok   intl/collator.tspp
ok   intl/common.tspp
ok   intl/date-time-format.tspp
ok   intl/display-names.tspp
ok   intl/duration-format.tspp
ok   intl/error.tspp
ok   intl/format.tspp
ok   intl/index.tspp
ok   intl/list-format.tspp
ok   intl/locale.tspp
ok   intl/number-format.tspp
ok   intl/plural-rules.tspp
ok   intl/relative-time-format.tspp
ok   intl/segmenter.tspp
ok   iter/index.tspp
ok   iter/iterator.tspp
ok   json/codec.tspp
ok   json/error.tspp
ok   json/index.tspp
ok   json/json.tspp
ok   json/value.tspp
ok   math/arithmetic.tspp
ok   math/bigint.tspp
ok   math/cast.tspp
ok   math/complex.tspp
ok   math/float.tspp
ok   math/identity.tspp
ok   math/index.tspp
ok   math/integer.tspp
ok   math/linear.tspp
ok   math/math.tspp
ok   math/matrix.tspp
ok   math/number.tspp
ok   math/numeric.tspp
ok   math/quaternion.tspp
ok   math/vector.tspp
ok   math/wrapping.tspp
ok   memory/access.tspp
ok   memory/arc/arc.tspp
ok   memory/arc/index.tspp
ok   memory/arc/weak.tspp
ok   memory/arena/arena.tspp
ok   memory/arena/index.tspp
ok   memory/binding/advise.tspp
ok   memory/binding/index.tspp
ok   memory/binding/layout.tspp
ok   memory/binding/lock.tspp
ok   memory/binding/map.tspp
ok   memory/binding/memory.tspp
ok   memory/binding/virtual.tspp
ok   memory/borrow.tspp
ok   memory/box.tspp
ok   memory/capability.tspp
ok   memory/cell/cell.tspp
ok   memory/cell/index.tspp
ok   memory/cell/refcell.tspp
ok   memory/cow/cow.tspp
ok   memory/cow/index.tspp
ok   memory/dispose.tspp
ok   memory/drop.tspp
ok   memory/dynamic.tspp
ok   memory/error.tspp
ok   memory/index.tspp
ok   memory/init.tspp
ok   memory/owned.tspp
ok   memory/phantom.tspp
ok   memory/pin.tspp
ok   memory/pointer.tspp
ok   memory/raw.tspp
ok   memory/rc/index.tspp
ok   memory/rc/rc.tspp
ok   memory/rc/weak.tspp
ok   memory/region.tspp
ok   memory/replace.tspp
ok   memory/swap.tspp
ok   memory/type.tspp
ok   module/config.tspp
ok   module/index.tspp
ok   module/meta.tspp
ok   net/binding/address.tspp
ok   net/binding/connectivity.tspp
ok   net/binding/index.tspp
ok   net/binding/interface.tspp
ok   net/binding/net.tspp
ok   net/binding/options.tspp
ok   net/binding/raw.tspp
ok   net/binding/resolve.tspp
ok   net/binding/route.tspp
ok   net/binding/socket.tspp
ok   net/binding/udp.tspp
ok   net/index.tspp
ok   net/net.tspp
ok   ops/bitwise.tspp
ok   ops/comparison.tspp
ok   ops/dereference.tspp
ok   ops/divide.tspp
ok   ops/equality.tspp
ok   ops/equate.tspp
ok   ops/format.tspp
ok   ops/hash.tspp
ok   ops/index.tspp
ok   ops/minus.tspp
ok   ops/multiply.tspp
ok   ops/negate.tspp
ok   ops/plus.tspp
ok   ops/power.tspp
ok   ops/remainder.tspp
ok   ops/shift.tspp
ok   ops/subscript.tspp
ok   ops/try.tspp
ok   prelude.tspp
ok   profile/index.tspp
ok   profile/instrument.tspp
ok   random/binding/entropy.tspp
ok   random/binding/index.tspp
ok   random/binding/random.tspp
ok   random/index.tspp
ok   random/random.tspp
ok   range/bound.tspp
ok   range/index.tspp
ok   range/iterator.tspp
ok   range/range.tspp
ok   range/step.tspp
ok   reflect/index.tspp
ok   reflect/type.tspp
ok   regexp/index.tspp
ok   regexp/regexp.tspp
ok   runtime/action.tspp
ok   runtime/binding.tspp
ok   runtime/control.tspp
ok   runtime/entity.tspp
ok   runtime/frame.tspp
ok   runtime/heap.tspp
ok   runtime/index.tspp
ok   runtime/inspect.tspp
ok   runtime/lineage.tspp
ok   runtime/observation.tspp
ok   runtime/policy.tspp
ok   runtime/random.tspp
ok   runtime/resource.tspp
ok   runtime/snapshot.tspp
ok   runtime/trace.tspp
ok   runtime/world.tspp
ok   serde/index.tspp
ok   serde/serde.tspp
ok   signal/accessor.tspp
ok   signal/action.tspp
ok   signal/control.tspp
ok   signal/derive.tspp
ok   signal/effect.tspp
ok   signal/index.tspp
ok   signal/map.tspp
ok   signal/memo.tspp
ok   signal/optimistic.tspp
ok   signal/options.tspp
ok   signal/owner.tspp
ok   signal/reaction.tspp
ok   signal/setter.tspp
ok   signal/signal.tspp
ok   stream/index.tspp
ok   stream/stream.tspp
ok   string/builder.tspp
ok   string/character.tspp
ok   string/cstring.tspp
ok   string/index.tspp
ok   string/os.tspp
ok   string/slice.tspp
ok   string/string.tspp
ok   string/utf8.tspp
ok   sync/atomic.tspp
ok   sync/barrier.tspp
ok   sync/condvar.tspp
ok   sync/index.tspp
ok   sync/mutex.tspp
ok   sync/once.tspp
ok   sync/reader.tspp
ok   sync/rwlock.tspp
ok   sync/seek.tspp
ok   sync/semaphore.tspp
ok   sync/writer.tspp
ok   telemetry/binding/index.tspp
ok   telemetry/binding/telemetry.tspp
ok   telemetry/field.tspp
ok   telemetry/index.tspp
ok   telemetry/log.tspp
ok   telemetry/metric.tspp
ok   telemetry/record.tspp
ok   telemetry/trace.tspp
ok   test/artifact.tspp
ok   test/body.tspp
ok   test/case.tspp
ok   test/context.tspp
FAIL test/expect.tspp :: error[unsupported-lower-construct]: unsupported construct: the 'error.catchUnwind' intrinsic
ok   test/fixture.tspp
ok   test/hook.tspp
ok   test/id.tspp
ok   test/index.tspp
ok   test/issue.tspp
ok   test/options.tspp
ok   test/poll.tspp
ok   test/replay.tspp
ok   test/run.tspp
ok   test/snapshot.tspp
ok   test/suite.tspp
ok   time/binding/clock.tspp
ok   time/binding/index.tspp
ok   time/binding/time.tspp
ok   time/binding/timer.tspp
ok   time/duration.tspp
ok   time/error.tspp
ok   time/index.tspp
ok   time/instant.tspp
ok   time/now.tspp
ok   time/option.tspp
ok   time/plain-date-time.tspp
ok   time/plain-date.tspp
ok   time/plain-month-day.tspp
ok   time/plain-time.tspp
ok   time/plain-year-month.tspp
ok   time/unit.tspp
ok   time/zoned-date-time.tspp
ok   topology/binding/edge.tspp
ok   topology/binding/entity.tspp
ok   topology/binding/index.tspp
ok   topology/binding/topology.tspp
ok   topology/decorator.tspp
ok   topology/edge.tspp
ok   topology/entity.tspp
ok   topology/index.tspp
ok   topology/label.tspp
ok   topology/topology.tspp
ok   tree/builder.tspp
ok   tree/index.tspp
ok   types/boolean.tspp
ok   types/function.tspp
ok   types/index.tspp
ok   types/object.tspp
ok   types/string.tspp
ok   worker/index.tspp
ok   worker/worker.tspp
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
