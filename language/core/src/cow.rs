use std::sync::Arc;

/// One contiguous copy on write buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CowBuffer<T> {
    /// The shared backing storage.
    values: Arc<Vec<T>>,
}

impl<T> Default for CowBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> CowBuffer<T> {
    /// Create one empty copy on write buffer.
    pub fn new() -> Self {
        Self {
            values: Arc::new(Vec::new()),
        }
    }

    /// Create one copy on write buffer from one owned vector.
    pub fn from_vec(values: Vec<T>) -> Self {
        Self {
            values: Arc::new(values),
        }
    }

    /// Return the number of stored values.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Borrow the whole buffer as one slice.
    pub fn as_slice(&self) -> &[T] {
        self.values.as_slice()
    }

    /// Ensure unique ownership and return the mutable backing vector.
    pub fn make_mut(&mut self) -> &mut Vec<T>
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.values)
    }

    /// Consume the buffer into one owned vector.
    pub fn into_vec(self) -> Vec<T>
    where
        T: Clone,
    {
        match Arc::try_unwrap(self.values) {
            Ok(values) => values,
            Err(values) => values.as_ref().clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CowBuffer;

    #[test]
    fn test_detach_on_mutation() {
        let mut left = CowBuffer::from_vec(vec![1, 2, 3]);
        let right = left.clone();

        // mutate one fork
        left.make_mut()[1] = 9;

        // keep the other fork unchanged
        assert_eq!(left.as_slice(), &[1, 9, 3]);
        assert_eq!(right.as_slice(), &[1, 2, 3]);
    }
}
