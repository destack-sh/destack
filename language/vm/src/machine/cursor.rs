use std::ptr::{self, NonNull};

use tspp_bytecode::{CodeOffset, Instruction};
use tspp_program::Word;

use super::Frame;

/// Native addresses for one active bytecode frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cursor {
    /// The first instruction byte in the active function.
    code: NonNull<u8>,
    /// The current instruction byte.
    instruction: NonNull<u8>,
    /// The current function-relative instruction byte.
    pc: CodeOffset,
    /// The first register in the active frame.
    registers: NonNull<Word>,
    /// The active canonical frame.
    frame: NonNull<Frame>,
}

/// One live position inside the active function's code.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Position {
    /// The current instruction byte.
    instruction: NonNull<u8>,
    /// The current function-relative instruction byte.
    pc: CodeOffset,
}

impl Cursor {
    /// Create an inactive cursor.
    pub(crate) const fn dangling() -> Self {
        Self {
            code: NonNull::dangling(),
            instruction: NonNull::dangling(),
            pc: CodeOffset(0),
            registers: NonNull::dangling(),
            frame: NonNull::dangling(),
        }
    }

    /// Point this cursor at one active function and register window.
    ///
    /// # Safety
    ///
    /// The addresses must remain valid until this cursor is replaced.
    pub(crate) unsafe fn set(
        &mut self,
        code: *const u8,
        pc: CodeOffset,
        registers: *mut Word,
        frame: *mut Frame,
    ) {
        // SAFETY: the caller guarantees the complete function and register window
        unsafe {
            self.code = NonNull::new_unchecked(code.cast_mut());
            self.instruction = NonNull::new_unchecked(code.add(pc.index()).cast_mut());
            self.pc = pc;
            self.registers = NonNull::new_unchecked(registers);
            self.frame = NonNull::new_unchecked(frame);
        }
    }

    /// Return the active canonical frame.
    #[inline(always)]
    pub(crate) fn frame(&self) -> Frame {
        // SAFETY: the cursor is replaced after every frame vector mutation
        let mut frame = unsafe { *self.frame.as_ref() };
        frame.pc = self.pc;

        frame
    }

    /// Return the active code position.
    #[inline(always)]
    pub(crate) const fn position(&self) -> Position {
        Position {
            instruction: self.instruction,
            pc: self.pc,
        }
    }

    /// Replace the active code position.
    #[inline(always)]
    pub(crate) fn set_position(&mut self, position: Position) {
        self.instruction = position.instruction;
        self.pc = position.pc;
    }

    /// Replace the active canonical frame.
    #[inline(always)]
    pub(crate) fn replace_frame(&mut self, frame: Frame) {
        self.pc = frame.pc;

        // SAFETY: the cursor retains exclusive activation access to the active frame
        unsafe {
            self.frame.as_ptr().write(frame);
        }
    }

    /// Save the live instruction offset in the active canonical frame.
    #[inline(always)]
    pub(crate) fn save_position(&mut self) {
        // SAFETY: the cursor retains exclusive activation access to the active frame
        unsafe {
            self.frame.as_mut().pc = self.pc;
        }
    }

    /// Branch relative to the current instruction successor.
    #[inline(always)]
    pub(crate) fn branch(&mut self, displacement: i32) {
        let mut position = self.position();
        position.branch(displacement);
        self.set_position(position);
    }

    /// Move to one function-relative byte offset.
    #[inline(always)]
    pub(crate) fn jump(&mut self, pc: CodeOffset) {
        self.pc = pc;
        self.jump_instruction(pc);
    }

    /// Move the instruction address without changing canonical state.
    #[inline(always)]
    fn jump_instruction(&mut self, pc: CodeOffset) {
        // SAFETY: linked branch targets remain inside the active function
        self.instruction = unsafe { NonNull::new_unchecked(self.code.as_ptr().add(pc.index())) };
    }

    /// Read one active register word.
    #[inline(always)]
    pub(crate) fn read(&self, register: u16) -> Word {
        // SAFETY: linked register indices remain inside the active frame window
        unsafe { ptr::read(self.registers.as_ptr().add(register as usize)) }
    }

    /// Write one active register word.
    #[inline(always)]
    pub(crate) fn write(&mut self, register: u16, value: Word) {
        // SAFETY: linked register indices remain inside the active frame window
        unsafe {
            ptr::write(self.registers.as_ptr().add(register as usize), value);
        }
    }

    /// Move one possibly overlapping active register range.
    #[inline(always)]
    pub(crate) fn move_words(&mut self, source: u16, target: u16, word_count: u16) {
        // SAFETY: linked register ranges remain inside the active frame window
        unsafe {
            ptr::copy(
                self.registers.as_ptr().add(source as usize),
                self.registers.as_ptr().add(target as usize),
                word_count as usize,
            );
        }
    }

    /// Copy active register words into one non-overlapping native destination.
    #[inline(always)]
    pub(crate) unsafe fn store_words(&self, source: u16, target: *mut Word, word_count: u16) {
        // SAFETY: the caller guarantees a writable non-overlapping destination
        unsafe {
            ptr::copy_nonoverlapping(
                self.registers.as_ptr().add(source as usize),
                target,
                word_count as usize,
            );
        }
    }

    /// Copy native source words into active result registers.
    #[inline(always)]
    pub(crate) unsafe fn load_words(&mut self, source: *const Word, target: u16, word_count: u16) {
        // SAFETY: the caller guarantees a readable non-overlapping source
        unsafe {
            ptr::copy_nonoverlapping(
                source,
                self.registers.as_ptr().add(target as usize),
                word_count as usize,
            );
        }
    }
}

impl Position {
    /// Return the current function-relative instruction offset.
    #[inline(always)]
    pub(crate) const fn pc(self) -> CodeOffset {
        self.pc
    }

    /// Read the current encoded instruction.
    ///
    /// # Safety
    ///
    /// The active Program must outlive the returned instruction.
    #[inline(always)]
    pub(crate) unsafe fn decode<'code>(self) -> Instruction<'code> {
        // SAFETY: linked Program code is produced by the bytecode builder
        unsafe { Instruction::read_raw(self.instruction.as_ptr()) }
    }

    /// Advance by one encoded instruction byte length.
    #[inline(always)]
    pub(crate) fn advance(&mut self, byte_len: usize) {
        self.pc.0 += byte_len as u32;

        // SAFETY: linked instruction lengths remain inside the active function
        self.instruction =
            unsafe { NonNull::new_unchecked(self.instruction.as_ptr().add(byte_len)) };
    }

    /// Branch relative to the current instruction successor.
    #[inline(always)]
    pub(crate) fn branch(&mut self, displacement: i32) {
        self.pc.0 = self.pc.0.wrapping_add_signed(displacement);

        // SAFETY: linked branch targets remain inside the active function
        self.instruction = unsafe {
            NonNull::new_unchecked(
                self.instruction
                    .as_ptr()
                    .wrapping_offset(displacement as isize),
            )
        };
    }
}
