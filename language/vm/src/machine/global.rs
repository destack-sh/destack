use destack_mir as mir;
use destack_program::{StaticAddress, StaticSpace};

use super::Activation;
use crate::diagnostic::Error;

impl Activation<'_> {
    /// Borrow local static memory for this activation.
    #[inline(always)]
    pub(crate) fn local_statics(&self) -> &StaticSpace {
        self.local_static
    }

    /// Borrow local static memory mutably for this activation.
    #[inline(always)]
    pub(crate) fn local_statics_mut(&mut self) -> &mut StaticSpace {
        self.local_static
    }

    /// Borrow shared static memory for this activation.
    #[inline(always)]
    pub(crate) fn shared_statics(&self) -> &StaticSpace {
        self.shared_static
    }

    /// Borrow shared static memory mutably for this activation.
    #[inline(always)]
    pub(crate) fn shared_statics_mut(&mut self) -> &mut StaticSpace {
        self.shared_static
    }

    /// Return the static address for one global.
    #[inline]
    pub(crate) fn static_address(
        &self,
        global: mir::LocalNodeId<mir::Global>,
    ) -> Option<StaticAddress> {
        self.local_statics()
            .address(self.machine.program.static_id(global))
            .or_else(|| {
                self.shared_statics()
                    .address(self.machine.program.static_id(global))
            })
            .or_else(|| self.machine.program.static_address(global))
    }

    /// Resolve one static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address(
        &self,
        address: StaticAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        self.local_statics()
            .native_address(address, byte_len)
            .or_else(|| self.shared_statics().native_address(address, byte_len))
            .or_else(|| {
                self.machine
                    .program
                    .constants()
                    .native_address(address, byte_len)
            })
            .ok_or(Error::invalid_instruction())
    }

    /// Resolve one mutable static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address_mut(
        &mut self,
        address: StaticAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        // prefer mutable local worker statics
        if let Some(native_address) = self
            .local_statics_mut()
            .native_address_mut(address, byte_len)
        {
            return Ok(native_address);
        }

        // then allow mutable shared statics
        if let Some(native_address) = self
            .shared_statics_mut()
            .native_address_mut(address, byte_len)
        {
            return Ok(native_address);
        }

        // reject writes into immutable local statics
        if self.local_statics().owns_address_range(address, byte_len) {
            return Err(Error::immutable_global_write(mir::LocalNodeId::new(
                address.id().0,
            )));
        }

        // reject writes into immutable shared statics
        if self.shared_statics().owns_address_range(address, byte_len) {
            return Err(Error::immutable_global_write(mir::LocalNodeId::new(
                address.id().0,
            )));
        }

        // reject writes into immutable program constants
        if self
            .machine
            .program
            .constants()
            .owns_address_range(address, byte_len)
        {
            return Err(Error::immutable_global_write(mir::LocalNodeId::new(
                address.id().0,
            )));
        }

        Err(Error::invalid_instruction())
    }
}
