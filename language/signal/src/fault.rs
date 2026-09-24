/// One synchronous fault delivered to the registered handlers.
#[derive(Debug)]
pub struct Fault {
    /// The faulting data address.
    address: usize,
    /// The interrupted thread context.
    context: *mut libc::ucontext_t,
}

impl Fault {
    /// Create one fault from the arguments of a signal handler.
    ///
    /// # Safety
    ///
    /// The siginfo and context pointers must come from one SA_SIGINFO delivery.
    pub(crate) unsafe fn new(siginfo: *mut libc::siginfo_t, context: *mut libc::c_void) -> Self {
        // SAFETY: the caller passes the delivered siginfo
        let address = unsafe { (*siginfo).si_addr() as usize };

        Self {
            address,
            context: context.cast(),
        }
    }

    /// Return the faulting data address.
    pub const fn address(&self) -> usize {
        self.address
    }

    /// Return the address of the faulting instruction.
    pub fn program_counter(&self) -> usize {
        // SAFETY: the context comes from the current signal delivery
        unsafe { Registers::of(self.context).program_counter() }
    }

    /// Resume the thread in a function as if the faulting frame's caller called it with one argument.
    ///
    /// # Safety
    ///
    /// The function must never return, and the faulting frame must hold only its frame record.
    pub unsafe fn call_from_caller(&mut self, function: usize, argument: usize) {
        // SAFETY: the context comes from the current signal delivery
        unsafe { Registers::of(self.context).call_from_caller(function, argument) }
    }

    /// Resume the thread in a function as if the faulting instruction called it with one argument.
    ///
    /// # Safety
    ///
    /// The function must never return, and the faulting frame must have saved its return address.
    pub unsafe fn call(&mut self, function: usize, argument: usize) {
        // SAFETY: the context comes from the current signal delivery
        unsafe { Registers::of(self.context).call(function, argument) }
    }
}

/// The interrupted general purpose registers of one platform.
#[cfg(all(target_vendor = "apple", target_arch = "aarch64"))]
type Registers = libc::__darwin_arm_thread_state64;
/// The interrupted general purpose registers of one platform.
#[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
type Registers = libc::__darwin_x86_thread_state64;
/// The interrupted general purpose registers of one platform.
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
type Registers = libc::mcontext_t;
/// The interrupted general purpose registers of one platform.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
type Registers = libc::mcontext_t;

/// The register access shared by every platform.
trait RegisterAccess {
    /// Borrow the registers inside one interrupted context.
    unsafe fn of<'a>(context: *mut libc::ucontext_t) -> &'a mut Self;

    /// Return the program counter.
    fn program_counter(&self) -> usize;

    /// Resume in a function called with one argument from the program counter.
    unsafe fn call(&mut self, function: usize, argument: usize);

    /// Pop the frame record and resume in a function called with one argument from the caller.
    unsafe fn call_from_caller(&mut self, function: usize, argument: usize);
}

#[cfg(all(target_vendor = "apple", target_arch = "aarch64"))]
impl RegisterAccess for Registers {
    unsafe fn of<'a>(context: *mut libc::ucontext_t) -> &'a mut Self {
        // SAFETY: darwin contexts point at one live machine context
        unsafe { &mut (*(*context).uc_mcontext).__ss }
    }

    fn program_counter(&self) -> usize {
        self.__pc as usize
    }

    unsafe fn call(&mut self, function: usize, argument: usize) {
        self.__lr = self.__pc + 1;
        self.__x[0] = argument as u64;
        self.__pc = function as u64;
    }

    unsafe fn call_from_caller(&mut self, function: usize, argument: usize) {
        let record = self.__fp as *const u64;

        // SAFETY: the caller guarantees a frame record at the frame pointer
        unsafe {
            self.__fp = *record;
            self.__lr = *record.add(1);
        }
        self.__sp = record as u64 + 16;
        self.__x[0] = argument as u64;
        self.__pc = function as u64;
    }
}

#[cfg(all(target_vendor = "apple", target_arch = "x86_64"))]
impl RegisterAccess for Registers {
    unsafe fn of<'a>(context: *mut libc::ucontext_t) -> &'a mut Self {
        // SAFETY: darwin contexts point at one live machine context
        unsafe { &mut (*(*context).uc_mcontext).__ss }
    }

    fn program_counter(&self) -> usize {
        self.__rip as usize
    }

    unsafe fn call(&mut self, function: usize, argument: usize) {
        // push the return address onto the interrupted stack
        self.__rsp -= 8;
        // SAFETY: the caller guarantees stack room below the interrupted frame
        unsafe { *(self.__rsp as *mut u64) = self.__rip + 1 };

        self.__rdi = argument as u64;
        self.__rip = function as u64;
    }

    unsafe fn call_from_caller(&mut self, function: usize, argument: usize) {
        let record = self.__rbp as *const u64;

        // SAFETY: the caller guarantees a frame record at the frame pointer
        unsafe { self.__rbp = *record };
        self.__rsp = record as u64 + 8;
        self.__rdi = argument as u64;
        self.__rip = function as u64;
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
impl RegisterAccess for Registers {
    unsafe fn of<'a>(context: *mut libc::ucontext_t) -> &'a mut Self {
        // SAFETY: linux contexts embed one machine context
        unsafe { &mut (*context).uc_mcontext }
    }

    fn program_counter(&self) -> usize {
        self.pc as usize
    }

    unsafe fn call(&mut self, function: usize, argument: usize) {
        self.regs[30] = self.pc + 1;
        self.regs[0] = argument as u64;
        self.pc = function as u64;
    }

    unsafe fn call_from_caller(&mut self, function: usize, argument: usize) {
        let record = self.regs[29] as *const u64;

        // SAFETY: the caller guarantees a frame record at the frame pointer
        unsafe {
            self.regs[29] = *record;
            self.regs[30] = *record.add(1);
        }
        self.sp = record as u64 + 16;
        self.regs[0] = argument as u64;
        self.pc = function as u64;
    }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl RegisterAccess for Registers {
    unsafe fn of<'a>(context: *mut libc::ucontext_t) -> &'a mut Self {
        // SAFETY: linux contexts embed one machine context
        unsafe { &mut (*context).uc_mcontext }
    }

    fn program_counter(&self) -> usize {
        self.gregs[libc::REG_RIP as usize] as usize
    }

    unsafe fn call(&mut self, function: usize, argument: usize) {
        let return_address = self.gregs[libc::REG_RIP as usize] + 1;

        // push the return address onto the interrupted stack
        self.gregs[libc::REG_RSP as usize] -= 8;
        let stack_pointer = self.gregs[libc::REG_RSP as usize];
        // SAFETY: the caller guarantees stack room below the interrupted frame
        unsafe { *(stack_pointer as *mut i64) = return_address };

        self.gregs[libc::REG_RDI as usize] = argument as i64;
        self.gregs[libc::REG_RIP as usize] = function as i64;
    }

    unsafe fn call_from_caller(&mut self, function: usize, argument: usize) {
        let record = self.gregs[libc::REG_RBP as usize] as *const i64;

        // SAFETY: the caller guarantees a frame record at the frame pointer
        unsafe { self.gregs[libc::REG_RBP as usize] = *record };
        self.gregs[libc::REG_RSP as usize] = record as i64 + 8;
        self.gregs[libc::REG_RDI as usize] = argument as i64;
        self.gregs[libc::REG_RIP as usize] = function as i64;
    }
}
