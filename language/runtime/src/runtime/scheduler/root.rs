use {destack_engine as engine, destack_heap as heap};

use super::EventLoop;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::engine::Engine;
use crate::runtime::heap::RootSink;

impl EventLoop {
    /// Visit GC roots retained by queued and suspended event-loop state.
    pub(crate) fn visit_roots(
        &mut self,
        engine: &mut Engine,
        roots: &mut RootSink<'_>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for task in &self.tasks {
            engine.visit_continuation_roots(&task.runnable, roots)?;
            visit_resume_value_roots(&task.resume_value, roots);
        }

        // queued microtasks
        for microtask in &self.microtasks {
            engine.visit_continuation_roots(&microtask.continuation, roots)?;
            visit_resume_value_roots(&microtask.resume_value, roots);
        }

        // suspended continuations
        for waiter in self.waiters.values() {
            engine.visit_continuation_image_roots(&waiter.runnable, roots)?;
            visit_resume_value_roots(&waiter.resume_value, roots);
        }

        Ok(())
    }

    /// Visit mutable local root slots retained by queued scheduler state.
    pub(crate) fn visit_root_slots(
        &mut self,
        engine: &mut Engine,
        visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
    ) -> RuntimeResult<()> {
        // queued tasks
        for task in &mut self.tasks {
            engine.visit_continuation_root_slots(&mut task.runnable, visit)?;
            visit_resume_value_root_slot(&mut task.resume_value, visit)?;
        }

        // queued microtasks
        for microtask in &mut self.microtasks {
            engine.visit_continuation_root_slots(&mut microtask.continuation, visit)?;
            visit_resume_value_root_slot(&mut microtask.resume_value, visit)?;
        }

        // suspended continuations
        for waiter in self.waiters.values_mut() {
            engine.visit_continuation_image_root_slots(&mut waiter.runnable, visit)?;
            visit_resume_value_root_slot(&mut waiter.resume_value, visit)?;
        }

        Ok(())
    }
}

/// Visit heap roots embedded in one resume value.
fn visit_resume_value_roots(value: &engine::Value, roots: &mut RootSink<'_>) {
    // direct heap roots
    match value {
        engine::Value::HeapReference(reference) => {
            roots.push_heap(*reference);
        }
        engine::Value::SharedHeapReference(reference) => {
            roots.push_shared_heap(*reference);
        }

        // non root payloads
        engine::Value::Void
        | engine::Value::Bool(_)
        | engine::Value::Int { .. }
        | engine::Value::UInt { .. }
        | engine::Value::Float32 { .. }
        | engine::Value::Float64 { .. }
        | engine::Value::Char(_)
        | engine::Value::RawPointer(_)
        | engine::Value::SharedRawPointer(_) => {}
    }
}

/// Visit the mutable local root slot in one value.
fn visit_resume_value_root_slot(
    value: &mut engine::Value,
    visit: &mut dyn FnMut(heap::RootSlot<'_>) -> heap::HeapResult<()>,
) -> RuntimeResult<()> {
    let engine::Value::HeapReference(reference) = value else {
        return Ok(());
    };

    visit(heap::RootSlot::Reference(reference)).map_err(Box::<RuntimeError>::from)
}
