use std::collections::VecDeque;

use parking_lot::Mutex;

use crate::CompileTask;

/// Queue of compiler tasks.
#[derive(Debug)]
pub struct CompilerQueue {
    queue: Mutex<VecDeque<CompileTask>>,
}

impl Default for CompilerQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerQueue {
    /// Create a new compiler task queue.
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Push a task to the back of the queue.
    pub fn push_back(&self, task: CompileTask) {
        self.queue.lock().push_back(task);
    }

    /// Pop a task from the front of the queue.
    pub fn pop_front(&self) -> Option<CompileTask> {
        self.queue.lock().pop_front()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }
}
