use destack_program::{GlobalAddress, GlobalId, GlobalLocation, StaticSpace};

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

    /// Return the global address for one global.
    #[inline]
    pub(crate) fn global_address(&self, global: GlobalId) -> Option<GlobalAddress> {
        self.program.global_address(global)
    }

    /// Resolve one static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address(
        &self,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        let global = self
            .program
            .global(address.global())
            .ok_or(Error::invalid_instruction())?;

        match global.location {
            GlobalLocation::LocalStatic => self
                .local_statics()
                .native_address(global, address, byte_len)
                .ok_or(Error::invalid_instruction()),
            GlobalLocation::SharedStatic => self
                .shared_statics()
                .native_address(global, address, byte_len)
                .ok_or(Error::invalid_instruction()),
            GlobalLocation::Constant => self
                .program
                .constant_native_address(address, byte_len)
                .ok_or(Error::invalid_instruction()),
        }
    }

    /// Resolve one mutable static byte range to a native address.
    #[inline]
    pub(crate) fn static_native_address_mut(
        &mut self,
        address: GlobalAddress,
        byte_len: usize,
    ) -> Result<usize, Error> {
        let global = self
            .program
            .global(address.global())
            .copied()
            .ok_or(Error::invalid_instruction())?;
        if !global.is_mutable() {
            return Err(Error::immutable_global_write(address.global()));
        }

        match global.location {
            GlobalLocation::LocalStatic => self
                .local_statics_mut()
                .native_address_mut(&global, address, byte_len)
                .map_err(Error::from)?
                .ok_or(Error::invalid_instruction()),
            GlobalLocation::SharedStatic => self
                .shared_statics_mut()
                .native_address_mut(&global, address, byte_len)
                .map_err(Error::from)?
                .ok_or(Error::invalid_instruction()),
            GlobalLocation::Constant => Err(Error::immutable_global_write(address.global())),
        }
    }
}
