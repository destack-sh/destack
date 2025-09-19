/// A school book stack.
/// Allows adding, removing, and inspecting nodes at the back.
pub(crate) trait Stack<T> {
    /// Removes the last node if any and gets it.
    fn pop(&mut self) -> Option<T>;

    /// Pushes a new node at the back.
    fn push(&mut self, value: T);

    /// Gets the last node if any.
    fn top(&self) -> Option<&T>;
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
}

/// A Stack that is stacked on top of another stack.
/// Guarantees that the underlying stack remains unchanged.
#[derive(Debug, Clone)]
pub(crate) struct StackedStack<'a, T> {
    /// The content of the original stack.
    original: std::slice::Iter<'a, T>,

    /// Items that have been pushed since the creation of this stack and aren't part of the `original` stack.
    stack: Vec<T>,
}

impl<'a, T> StackedStack<'a, T> {
    #[cfg(test)]
    pub(crate) fn new(original: &'a [T]) -> Self {
        Self::with_vec(original, Vec::new())
    }

    /// Creates a new stack that uses `stack` for storing its nodes.
    pub(crate) fn with_vec(original: &'a [T], stack: Vec<T>) -> Self {
        Self {
            original: original.iter(),
            stack,
        }
    }

    /// Gets the underlying `stack` vector.
    pub(crate) fn into_vec(self) -> Vec<T> {
        self.stack
    }
}

impl<T> Stack<T> for StackedStack<'_, T>
where
    T: Copy,
{
    fn pop(&mut self) -> Option<T> {
        self.stack
            .pop()
            .or_else(|| self.original.next_back().copied())
    }

    fn push(&mut self, value: T) {
        self.stack.push(value);
    }

    fn top(&self) -> Option<&T> {
        self.stack
            .last()
            .or_else(|| self.original.as_slice().last())
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
        // partially consume stack then add more nodes
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
        // add multiple nodes then pop some of them
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
