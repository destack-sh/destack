use crossbeam_deque::Worker;

use crate::CompilerTask;

/// Queue of compiler tasks using a crossbeam_deque Worker.
#[derive(Debug)]
pub struct CompilerQueue {
    local: Worker<CompilerTask>,
}

impl Default for CompilerQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerQueue {
    /// Create a new compiler task queue (LIFO).
    pub fn new() -> Self {
        Self {
            local: Worker::new_fifo(),
        }
    }

    /// Push a task to the back of the queue.
    pub fn push_back(&self, task: CompilerTask) {
        self.local.push(task);
    }

    /// Pop a task from the front of the queue.
    pub fn pop_front(&self) -> Option<CompilerTask> {
        self.local.pop()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
    }
}
