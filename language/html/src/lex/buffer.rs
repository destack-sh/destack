use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;

use crate::lex::HtmlString;

pub(crate) use self::SetResult::{FromSet, NotFromSet};
use crate::lex::SmallCharSet;

/// One result from `pop_except_from`.
#[derive(PartialEq, Eq, Debug)]
pub(crate) enum SetResult {
    /// A character from the `SmallCharSet`.
    FromSet(char),
    /// A string buffer containing no characters from the `SmallCharSet`.
    NotFromSet(HtmlString),
}

/// One queue of owned HTML string chunks.
#[derive(Clone, Debug)]
pub(crate) struct BufferQueue {
    /// Buffers to process.
    buffers: RefCell<VecDeque<HtmlString>>,
}

impl Default for BufferQueue {
    /// Create one empty buffer queue.
    #[inline]
    fn default() -> Self {
        Self {
            buffers: RefCell::new(VecDeque::with_capacity(16)),
        }
    }
}

impl BufferQueue {
    /// Return whether the queue is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.buffers.borrow().is_empty()
    }

    /// Pop the first string chunk from the queue.
    #[inline]
    pub(crate) fn pop_front(&self) -> Option<HtmlString> {
        self.buffers.borrow_mut().pop_front()
    }

    /// Push one string chunk onto the front of the queue.
    pub(crate) fn push_front(&self, buffer: HtmlString) {
        // ignore empty chunks
        if buffer.len32() == 0 {
            return;
        }

        self.buffers.borrow_mut().push_front(buffer);
    }

    /// Push one string chunk onto the back of the queue.
    pub(crate) fn push_back(&self, buffer: HtmlString) {
        // ignore empty chunks
        if buffer.len32() == 0 {
            return;
        }

        self.buffers.borrow_mut().push_back(buffer);
    }

    /// Return the next available character without consuming it.
    pub(crate) fn peek(&self) -> Option<char> {
        debug_assert!(
            !self.buffers.borrow().iter().any(|el| el.len32() == 0),
            "invariant \"all buffers in the queue are non-empty\" failed"
        );

        // front character
        self.buffers
            .borrow()
            .front()
            .and_then(|buffer| buffer.chars().next())
    }

    /// Pop one matching character or one run outside the set.
    pub(crate) fn pop_except_from(&self, set: SmallCharSet) -> Option<SetResult> {
        let (result, now_empty) = match self.buffers.borrow_mut().front_mut() {
            None => (None, false),
            Some(buffer) => {
                let n = set.nonmember_prefix_len(buffer);

                // nonmatching prefix
                if n > 0 {
                    let out;

                    // matched run
                    // SAFETY: n is a prefix byte length returned by the same buffer
                    unsafe {
                        out = buffer.unsafe_substring(0, n);
                        buffer.unsafe_pop_front(n);
                    }
                    (Some(NotFromSet(out)), buffer.is_empty())
                } else {
                    let c = buffer.pop_front_char()?;
                    (Some(FromSet(c)), buffer.is_empty())
                }
            }
        };

        // drop empty chunks
        if now_empty {
            self.buffers.borrow_mut().pop_front();
        }

        result
    }

    /// Consume one byte pattern with one custom byte comparison.
    pub(crate) fn eat<F: Fn(&u8, &u8) -> bool>(&self, pat: &str, eq: F) -> Option<bool> {
        let mut buffers_exhausted = 0;
        let mut consumed_from_last = 0;

        // empty queue
        self.buffers.borrow().front()?;

        // match the pattern across chunks
        for pattern_byte in pat.bytes() {
            if buffers_exhausted >= self.buffers.borrow().len() {
                return None;
            }
            let buffer = &self.buffers.borrow()[buffers_exhausted];

            if !eq(&buffer.as_bytes()[consumed_from_last], &pattern_byte) {
                return Some(false);
            }

            consumed_from_last += 1;
            if consumed_from_last >= buffer.len() {
                buffers_exhausted += 1;
                consumed_from_last = 0;
            }
        }

        // commit the matched prefix
        for _ in 0..buffers_exhausted {
            self.buffers.borrow_mut().pop_front();
        }

        // final partial chunk
        match self.buffers.borrow_mut().front_mut() {
            None => assert_eq!(consumed_from_last, 0),
            Some(ref mut buffer) => buffer.pop_front(consumed_from_last as u32),
        }

        Some(true)
    }

    /// Pop the next available character.
    pub(crate) fn next(&self) -> Option<char> {
        let (result, now_empty) = match self.buffers.borrow_mut().front_mut() {
            None => (None, false),
            Some(buffer) => {
                let c = buffer.pop_front_char()?;
                (Some(c), buffer.is_empty())
            }
        };

        // drop empty chunks
        if now_empty {
            self.buffers.borrow_mut().pop_front();
        }

        result
    }

    /// Return a mutable reference to the first string chunk in the queue.
    pub(crate) fn peek_front_chunk_mut(&self) -> Option<RefMut<'_, HtmlString>> {
        let buffers = self.buffers.borrow_mut();

        // empty queue
        if buffers.is_empty() {
            return None;
        }

        // first chunk
        let Ok(front_buffer) = RefMut::filter_map(buffers, |buffers| buffers.front_mut()) else {
            return None;
        };

        Some(front_buffer)
    }
}
