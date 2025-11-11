use std::collections::VecDeque;

use crate::CompilerTask;

/// Queue of compiler tasks.
#[derive(Debug, Clone)]
pub struct CompilerQueue {
    tasks: VecDeque<CompilerTask>,
}

impl Default for CompilerQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl CompilerQueue {
    /// Create a new empty queue.
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    /// Push a task to the back of the queue.
    pub fn push_back(&mut self, task: CompilerTask) {
        self.tasks.push_back(task);
    }

    /// Pop a task from the front of the queue.
    pub fn pop_front(&mut self) -> Option<CompilerTask> {
        self.tasks.pop_front()
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}
