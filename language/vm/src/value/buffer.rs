use serde::{Deserialize, Serialize};

use super::Word;

/// The inline byte budget for one word buffer.
///
/// 32 bytes keeps small local and raw word buffers inline without making the
/// buffer disproportionately large in interpreter hot paths.
/// This is a representation budget, not a runtime tuning option.
const INLINE_WORD_BUFFER_TARGET_BYTES: usize = 32;

/// The number of words kept inline in one word buffer.
const INLINE_WORD_BUFFER_VALUES: usize =
    INLINE_WORD_BUFFER_TARGET_BYTES / std::mem::size_of::<Word>();

/// One inline-optimized local word buffer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordBuffer {
    /// Inline words for small word counts.
    Inline {
        /// Number of words currently in use.
        len: u8,
        /// Inline word buffer.
        words: [Word; INLINE_WORD_BUFFER_VALUES],
    },
    /// Heap-allocated word buffer.
    Heap(Box<[Word]>),
}

impl Default for WordBuffer {
    fn default() -> Self {
        Self::Inline {
            len: 0,
            words: [Word::VOID; INLINE_WORD_BUFFER_VALUES],
        }
    }
}

impl WordBuffer {
    /// Create a new empty word buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create one word buffer with the given number of words.
    pub fn with_words_len(count: usize) -> Self {
        if count <= INLINE_WORD_BUFFER_VALUES {
            Self::Inline {
                len: count as u8,
                words: [Word::VOID; INLINE_WORD_BUFFER_VALUES],
            }
        } else {
            Self::Heap(vec![Word::VOID; count].into_boxed_slice())
        }
    }

    /// Create one word buffer from one list of words.
    pub fn with_words(words: Vec<Word>) -> Self {
        let len = words.len();

        // inline words
        if len <= INLINE_WORD_BUFFER_VALUES {
            let mut inline_words = [Word::VOID; INLINE_WORD_BUFFER_VALUES];
            for (index, word) in words.into_iter().enumerate() {
                inline_words[index] = word;
            }
            Self::Inline {
                len: len as u8,
                words: inline_words,
            }
        }
        // heap words
        else {
            Self::Heap(words.into_boxed_slice())
        }
    }

    /// Create one word buffer with exactly two words.
    #[inline]
    pub fn with_pair(first: Word, second: Word) -> Self {
        let mut words = [Word::VOID; INLINE_WORD_BUFFER_VALUES];
        words[0] = first;
        words[1] = second;

        Self::Inline { len: 2, words }
    }

    /// Create one word buffer with exactly one word.
    #[inline]
    pub fn with_single(word: Word) -> Self {
        let mut words = [Word::VOID; INLINE_WORD_BUFFER_VALUES];
        words[0] = word;

        Self::Inline { len: 1, words }
    }

    /// Return the retained heap bytes owned by this word buffer outside its inline form.
    pub fn retained_bytes(&self) -> usize {
        match self {
            Self::Inline { .. } => 0,
            Self::Heap(words) => std::mem::size_of_val(words.as_ref()),
        }
    }

    /// Return the number of words in this word buffer.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len as usize,
            Self::Heap(words) => words.len(),
        }
    }

    /// Report whether this word buffer has no words.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the word slice for this word buffer.
    #[inline]
    pub fn as_slice(&self) -> &[Word] {
        match self {
            Self::Inline { len, words } => &words[..*len as usize],
            Self::Heap(words) => words.as_ref(),
        }
    }

    /// Return one word by index.
    #[inline]
    pub fn get(&self, index: usize) -> Option<&Word> {
        self.as_slice().get(index)
    }

    /// Return one mutable word by index.
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Word> {
        match self {
            // inline words
            Self::Inline { len, words } => {
                if index < *len as usize {
                    Some(&mut words[index])
                } else {
                    None
                }
            }

            // heap words
            Self::Heap(words) => words.get_mut(index),
        }
    }

    /// Return one word without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &Word {
        match self {
            Self::Inline { len, words } => {
                debug_assert!(index < *len as usize, "inline word out of bounds");
                unsafe { words.get_unchecked(index) }
            }
            Self::Heap(words) => unsafe { words.get_unchecked(index) },
        }
    }

    /// Return one mutable word without bounds checks.
    ///
    /// # Safety
    /// Caller must ensure the index is within the current bounds.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut Word {
        match self {
            Self::Inline { len, words } => {
                debug_assert!(index < *len as usize, "inline word out of bounds");
                unsafe { words.get_unchecked_mut(index) }
            }
            Self::Heap(words) => unsafe { words.get_unchecked_mut(index) },
        }
    }

    /// Return the inline words when this word buffer is stored inline.
    #[inline]
    pub fn inline_words(&self) -> Option<&[Word]> {
        match self {
            Self::Inline { len, words } => Some(&words[..*len as usize]),
            Self::Heap(_) => None,
        }
    }

    /// Return the mutable inline words when this word buffer is stored inline.
    #[inline]
    pub fn inline_words_mut(&mut self) -> Option<&mut [Word]> {
        match self {
            Self::Inline { len, words } => Some(&mut words[..*len as usize]),
            Self::Heap(_) => None,
        }
    }

    /// Resize the local word buffer for this word buffer.
    pub fn resize(&mut self, new_len: usize, word: Word) {
        match self {
            // inline words
            Self::Inline { len, words } => {
                let old_len = *len as usize;

                // stay inline
                if new_len <= INLINE_WORD_BUFFER_VALUES {
                    if new_len > old_len {
                        words[old_len..new_len].fill(word);
                    }
                    *len = new_len as u8;
                    return;
                }

                // spill to heap
                let mut heap_words = Vec::with_capacity(new_len);
                heap_words.extend_from_slice(&words[..old_len]);
                heap_words.resize(new_len, word);
                *self = Self::Heap(heap_words.into_boxed_slice());
            }

            // heap words
            Self::Heap(words) => {
                let mut heap_words = words.to_vec();
                heap_words.resize(new_len, word);
                *words = heap_words.into_boxed_slice();
            }
        }
    }

    /// Append one local word to this word buffer.
    pub fn push(&mut self, word: Word) {
        match self {
            // inline words
            Self::Inline { len, words } => {
                let index = *len as usize;

                // keep words inline
                if index < INLINE_WORD_BUFFER_VALUES {
                    words[index] = word;
                    *len += 1;
                    return;
                }

                // spill to heap
                let mut heap_words = Vec::with_capacity(index + 1);
                heap_words.extend_from_slice(&words[..index]);
                heap_words.push(word);
                *self = Self::Heap(heap_words.into_boxed_slice());
            }

            // heap words
            Self::Heap(words) => {
                let mut heap_words = words.to_vec();
                heap_words.push(word);
                *words = heap_words.into_boxed_slice();
            }
        }
    }

    /// Clone this word buffer for a forked continuation.
    pub fn clone_for_fork(&self) -> Self {
        self.clone()
    }
}

impl<'a> IntoIterator for &'a WordBuffer {
    type Item = &'a Word;
    type IntoIter = std::slice::Iter<'a, Word>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
