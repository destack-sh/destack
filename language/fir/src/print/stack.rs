/// Stack operations shared by owned and restorable stack storage.
pub(crate) trait Stack<T> {
    /// Remove and return the final value.
    fn pop(&mut self) -> Option<T>;

    /// Append one value.
    fn push(&mut self, value: T);

    /// Return the final value.
    fn top(&self) -> Option<&T>;

    /// Return the final value mutably.
    fn top_mut(&mut self) -> Option<&mut T>;
}

impl<T> Stack<T> for Vec<T> {
    #[inline]
    fn pop(&mut self) -> Option<T> {
        self.pop()
    }

    #[inline]
    fn push(&mut self, value: T) {
        self.push(value);
    }

    #[inline]
    fn top(&self) -> Option<&T> {
        self.last()
    }

    #[inline]
    fn top_mut(&mut self) -> Option<&mut T> {
        self.last_mut()
    }
}

/// A mutable stack layered over one immutable borrowed stack.
#[derive(Debug, Clone)]
pub(crate) struct StackedStack<'a, T> {
    /// The unconsumed original values.
    original: std::slice::Iter<'a, T>,
    /// The materialized or newly pushed values.
    stack: Vec<T>,
}

impl<'a, T> StackedStack<'a, T> {
    #[cfg(test)]
    pub(crate) fn new(original: &'a [T]) -> Self {
        Self::with_vec(original, Vec::new())
    }

    /// Create a stack that borrows existing values and owns appended values.
    pub(crate) fn with_vec(original: &'a [T], stack: Vec<T>) -> Self {
        Self {
            original: original.iter(),
            stack,
        }
    }

    /// Take the reusable mutable storage.
    pub(crate) fn take_vec(&mut self) -> Vec<T> {
        std::mem::take(&mut self.stack)
    }
}

impl<T> Stack<T> for StackedStack<'_, T>
where
    T: Clone,
{
    fn pop(&mut self) -> Option<T> {
        self.stack
            .pop()
            .or_else(|| self.original.next_back().cloned())
    }

    fn push(&mut self, value: T) {
        self.stack.push(value);
    }

    fn top(&self) -> Option<&T> {
        self.stack
            .last()
            .or_else(|| self.original.as_slice().last())
    }

    fn top_mut(&mut self) -> Option<&mut T> {
        // materialize the next borrowed frame before mutating it
        if self.stack.is_empty()
            && let Some(value) = self.original.next_back().cloned()
        {
            self.stack.push(value);
        }

        self.stack.last_mut()
    }
}

#[cfg(test)]
mod tests {
    use crate::print::stack::{Stack, StackedStack};

    #[test]
    fn test_restore_consumed_stack() {
        // consume entire stack and verify original remains unchanged
        let original = vec![1, 2, 3];
        let mut restorable = StackedStack::new(&original);

        restorable.push(4);

        assert_eq!(restorable.pop(), Some(4));
        assert_eq!(restorable.pop(), Some(3));
        assert_eq!(restorable.pop(), Some(2));
        assert_eq!(restorable.pop(), Some(1));
        assert_eq!(restorable.pop(), None);

        assert_eq!(original, vec![1, 2, 3]);
    }

    #[test]
    fn test_restore_partially_consumed_stack() {
        // partially consume the stack, then add more values
        let original = vec![1, 2, 3];
        let mut restorable = StackedStack::new(&original);

        restorable.push(4);

        assert_eq!(restorable.pop(), Some(4));
        assert_eq!(restorable.pop(), Some(3));
        assert_eq!(restorable.pop(), Some(2));
        restorable.push(5);
        restorable.push(6);
        restorable.push(7);

        assert_eq!(original, vec![1, 2, 3]);
    }

    #[test]
    fn test_restore_stack() {
        // add multiple values, then pop some of them
        let original = vec![1, 2, 3];
        let mut restorable = StackedStack::new(&original);

        restorable.push(4);
        restorable.push(5);
        restorable.push(6);
        restorable.push(7);

        assert_eq!(restorable.pop(), Some(7));
        assert_eq!(restorable.pop(), Some(6));
        assert_eq!(restorable.pop(), Some(5));

        assert_eq!(original, vec![1, 2, 3]);
    }
}
