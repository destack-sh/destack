use destack_mir as mir;
use destack_program as program;

use super::Activation;
use crate::diagnostic::Error;
use destack_program::vm::{
    Check, CheckId, Edge, EdgeId, Instruction, Projection, ProjectionId, SideRecord, SideTable,
    SliceProjection, SliceProjectionId, SwitchCase, SwitchCasesId, SwitchTable, SwitchTableId,
    TensorConvolutionId, TensorDotId, TensorGatherId, TensorLayout, TensorLayoutId,
    TensorScatterId, TensorWindowId, U32RangeId,
};

impl Activation<'_> {
    /// Borrow one pooled side record.
    #[inline(always)]
    pub(crate) fn side<T: SideRecord + 'static>(&self, instruction: &Instruction) -> &'static T {
        let record = T::get(self.side_table(), instruction.a);

        // SAFETY: side records live in the machine program for the duration of dispatch
        unsafe { &*(record as *const T) }
    }

    /// Borrow one pooled side record by id.
    #[inline(always)]
    pub(crate) fn side_record<T: SideRecord + 'static>(&self, id: u32) -> &'static T {
        let record = T::get(self.side_table(), id);

        // SAFETY: side records live in the machine program for the duration of dispatch
        unsafe { &*(record as *const T) }
    }

    /// Return the canonical layout for one MIR type.
    #[inline]
    pub(crate) fn require_layout(&self, ty: program::TypeId) -> Result<&program::Layout, Error> {
        self.machine
            .program
            .layout(ty)
            .ok_or_else(|| Error::type_mismatch("compiled layout", format!("{ty:?}")))
    }

    /// Clear the last fallible allocation failure.
    #[inline]
    pub(crate) fn clear_allocation_failure(&mut self) {
        self.machine.last_allocation_failure = None;
    }

    /// Record one fallible allocation failure.
    #[inline]
    pub(crate) fn set_allocation_failure(&mut self, error: Error) {
        self.machine.last_allocation_failure = Some(error);
    }

    /// Borrow the program side table.
    #[inline(always)]
    pub(crate) fn side_table(&self) -> &'static SideTable {
        let side_table = self.machine.program.side_table();

        // SAFETY: side tables live in the machine program for the duration of dispatch
        unsafe { &*(side_table as *const SideTable) }
    }

    /// Return one pooled address projection.
    #[inline(always)]
    pub(crate) fn projection(&self, id: ProjectionId) -> Projection {
        *self.side_table().projection(id)
    }

    /// Return one pooled slice projection.
    #[inline(always)]
    pub(crate) fn slice_projection(&self, id: SliceProjectionId) -> SliceProjection {
        *self.side_table().slice_projection(id)
    }

    /// Borrow one pooled check constraint.
    #[inline(always)]
    pub(crate) fn check(&self, id: CheckId) -> &'static Check {
        self.side_table().check(id)
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub(crate) fn edge(&self, id: EdgeId) -> Edge {
        self.side_table().edge(id)
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &'static [SwitchCase] {
        self.side_table().switch_cases(id)
    }

    /// Borrow one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> &'static SwitchTable {
        self.side_table().switch_table(id)
    }

    /// Borrow one pooled u32 range.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &'static [u32] {
        self.side_table().u32_range(id)
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub(crate) fn tensor_layout(&self, id: TensorLayoutId) -> &'static TensorLayout {
        self.side_table().tensor_layout(id)
    }

    /// Borrow one pooled tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> &'static mir::TensorDotDimensionNumbers {
        self.side_table().tensor_dot(id)
    }

    /// Borrow one pooled tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> &'static mir::TensorConvolutionDimensionNumbers {
        self.side_table().tensor_convolution(id)
    }

    /// Borrow one pooled tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(
        &self,
        id: TensorWindowId,
    ) -> &'static mir::TensorConvolutionWindow {
        self.side_table().tensor_window(id)
    }

    /// Borrow one pooled tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(
        &self,
        id: TensorGatherId,
    ) -> &'static mir::TensorGatherDimensionNumbers {
        self.side_table().tensor_gather(id)
    }

    /// Borrow one pooled tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(
        &self,
        id: TensorScatterId,
    ) -> &'static mir::TensorScatterDimensionNumbers {
        self.side_table().tensor_scatter(id)
    }
}
