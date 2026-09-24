use std::hint::black_box;
use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_program as program;
use destack_repository::RuntimeOptions;
use destack_vm as vm;

use crate::binding::BindingTable;
use crate::tests::{TestProgram, TestWorker, TestWorld};
use crate::world::RunOutcome;

use super::super::{Loader, Platform};

/// Write a native frame local through its borrow in a callee and read the local back.
#[test]
fn test_write_a_native_frame_local_through_its_borrow() {
    let program = TestProgram::mir(
        r#"
function bump<'a>(v0: ref<int32, borrowed, 'a, mutable>): void {
entry(v0: ref<int32, borrowed, 'a, mutable>):
    v1: int32 = load (*v0)
    v2: int32 = 1
    v3: int32 = add v1, v2
    store (*v0), v3
    return
}

export function task(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, mutable> = address l0
    call bump(v1): (ref<int32, borrowed, 'frame, mutable>) => void
    v2: int32 = load l0
    return v2
}
"#,
    )
    .compile_native()
    .build();
    let code = Platform
        .load(&program)
        .expect("native test program should load");
    let mut worker = TestWorker::native(
        &RuntimeOptions::default(),
        program,
        BindingTable::new(),
        code,
    );

    let value = worker
        .run_entrypoint("task", 41)
        .expect("native frame borrow should execute");

    assert_eq!(value.words(), &[program::Word::int32(42)]);
}

/// Move a frame borrow from a stopped native frame into the VM frame that resumes it.
#[test]
fn test_move_a_frame_borrow_from_native_into_the_vm() {
    let program = TestProgram::mir(
        r#"
export function task(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v1: ref<int32, borrowed, 'frame, mutable> = address l0
    breakpoint
    v2: int32 = 7
    store (*v1), v2
    v3: int32 = load l0
    breakpoint
    return v3
}
"#,
    )
    .compile_native();
    let mut world = TestWorld::build(&RuntimeOptions::default(), program);
    let worker_id = world.default_worker_id();
    world.enqueue_task(worker_id, "task", 1);

    // stop natively with the borrow live, then resume in the VM up to the second stop
    let stop = world.run_to_stop();
    let RunOutcome::Stopped { stop } = world.resume(stop) else {
        panic!("resumed task should reach its second breakpoint");
    };
    assert_eq!(
        stop.reason,
        program::StopReason::Instruction {
            point: world.point("task", 6)
        }
    );

    // read the value the VM wrote through the moved borrow
    let image = world
        .world_mut()
        .image()
        .expect("stopped world should be inspectable");
    let frames = image
        .worker_frames(worker_id)
        .expect("stopped worker should be inspectable");
    let [frame] = frames.as_slice() else {
        panic!("stopped worker should retain one frame");
    };

    assert_eq!(frame.bytes.as_slice(), 7i32.to_le_bytes());
}

/// The world memory the fork test reserves.
const WORLD_BYTES: usize = 64 * 1024 * 1024;
/// The native stack the fork test reserves.
const NATIVE_STACK_BYTES: usize = 1024 * 1024;
/// The frame bytes each run writes near the top of its native stack.
const FRAME_BYTES: usize = 64 * 1024;

/// Run on a fiber's native stack again in both halves of a forked world, each half keeping its frame bytes.
#[test]
fn test_run_on_a_native_stack_after_a_world_fork() {
    let memory = Arc::new(MemoryMap::reserve(WORLD_BYTES, 0).expect("world memory should reserve"));
    let mut fiber = vm::Fiber::new(memory.clone(), FRAME_BYTES).expect("fiber should reserve");
    let stack = fiber
        .native_stack(NATIVE_STACK_BYTES)
        .expect("native stack should reserve");
    write_frame(stack, 1);

    // share every written page between the parent and the fork
    let forked = Arc::new(memory.fork_lazy().expect("world memory should fork"));
    let mut child = fiber.fork(forked.clone());

    // write the shared pages from frames standing on them in both halves
    let child_stack = child
        .native_stack(NATIVE_STACK_BYTES)
        .expect("forked native stack should exist");
    let child_frame = write_frame(child_stack, 2);
    let parent_stack = fiber
        .native_stack(NATIVE_STACK_BYTES)
        .expect("native stack should exist");
    let parent_frame = write_frame(parent_stack, 3);

    // read each half's frame back from its world
    let child_bytes = forked
        .read_bytes(child_frame, FRAME_BYTES)
        .expect("forked frame should read");
    let parent_bytes = memory
        .read_bytes(parent_frame, FRAME_BYTES)
        .expect("frame should read");
    assert_eq!(child_bytes, vec![2; FRAME_BYTES]);
    assert_eq!(parent_bytes, vec![3; FRAME_BYTES]);
}

/// Fill one frame's bytes on a native stack from code running on it and return the frame's world offset.
fn write_frame(stack: &vm::NativeStack, byte: u8) -> usize {
    let bottom = stack.bottom() as usize;
    let world_bottom = stack.world_range().start;

    // SAFETY: the native stack is a mapped world range the fiber holds for the whole call
    let address = unsafe {
        psm::on_stack(stack.bottom(), stack.byte_len(), || {
            let mut frame = [0u8; FRAME_BYTES];
            black_box(&mut frame).fill(byte);

            frame.as_ptr() as usize
        })
    };

    world_bottom + address - bottom
}
