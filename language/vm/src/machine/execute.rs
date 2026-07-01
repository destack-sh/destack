use destack_program as program;

use super::Activation;
use crate::diagnostic::Error;
use destack_core::SectionImage;
use destack_program::vm::{
    Check, CheckId, ConstValueId, Edge, EdgeId, Instruction, Projection, ProjectionId, SideRecord,
    SideTable, SignatureId, SliceProjection, SliceProjectionId, SwitchCase, SwitchCasesId,
    SwitchTable, SwitchTableId, TensorConvolutionDimensions, TensorConvolutionId,
    TensorConvolutionWindow, TensorDotDimensions, TensorDotId, TensorGatherDimensions,
    TensorGatherId, TensorLayoutId, TensorLayoutView, TensorScatterDimensions, TensorScatterId,
    TensorWindowId, U32RangeId,
};

impl Activation<'_> {
    /// Borrow one pooled side record.
    #[inline(always)]
    pub(crate) fn side<T: SideRecord>(&self, instruction: &Instruction) -> &T {
        T::get(self.side_table(), self.sections(), instruction.a)
    }

    /// Borrow one pooled side record by id.
    #[inline(always)]
    pub(crate) fn side_record<T: SideRecord>(&self, id: u32) -> &T {
        T::get(self.side_table(), self.sections(), id)
    }

    /// Return the canonical layout for one MIR type.
    #[inline]
    pub(crate) fn require_layout(&self, ty: program::TypeId) -> Result<&program::Layout, Error> {
        self.program
            .layout(ty)
            .ok_or_else(|| Error::type_mismatch("lowered layout", format!("{ty:?}")))
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

    /// Borrow program sections.
    #[inline(always)]
    pub(crate) fn sections(&self) -> SectionImage<'_> {
        self.program.sections()
    }

    /// Borrow the program side table.
    #[inline(always)]
    pub(crate) fn side_table(&self) -> &SideTable {
        self.program.side_table()
    }

    /// Return one pooled address projection.
    #[inline(always)]
    pub(crate) fn projection(&self, id: ProjectionId) -> Projection {
        self.side_table().projection(self.sections(), id)
    }

    /// Return one pooled slice projection.
    #[inline(always)]
    pub(crate) fn slice_projection(&self, id: SliceProjectionId) -> SliceProjection {
        self.side_table().slice_projection(self.sections(), id)
    }

    /// Return one pooled check constraint.
    #[inline(always)]
    pub(crate) fn check(&self, id: CheckId) -> Check {
        self.side_table().check(self.sections(), id)
    }

    /// Return one pooled control edge.
    #[inline(always)]
    pub(crate) fn edge(&self, id: EdgeId) -> Edge {
        self.side_table().edge(self.sections(), id)
    }

    /// Borrow one pooled switch case table.
    #[inline(always)]
    pub(crate) fn switch_cases(&self, id: SwitchCasesId) -> &[SwitchCase] {
        self.side_table().switch_cases(self.sections(), id)
    }

    /// Return one pooled dense switch table.
    #[inline(always)]
    pub(crate) fn switch_table(&self, id: SwitchTableId) -> SwitchTable {
        self.side_table().switch_table(self.sections(), id)
    }

    /// Borrow one pooled dense switch table's cases.
    #[inline(always)]
    pub(crate) fn switch_table_cases(&self, table: SwitchTable) -> &[SwitchCase] {
        self.side_table().switch_table_cases(self.sections(), table)
    }

    /// Borrow one pooled u32 range.
    #[inline(always)]
    pub(crate) fn u32_range(&self, id: U32RangeId) -> &[u32] {
        self.side_table().u32_range(self.sections(), id)
    }

    /// Borrow one pooled tensor layout.
    #[inline(always)]
    pub(crate) fn tensor_layout(&self, id: TensorLayoutId) -> TensorLayoutView<'_> {
        self.side_table().tensor_layout(self.sections(), id)
    }

    /// Return one pooled tensor dot descriptor.
    #[inline(always)]
    pub(crate) fn tensor_dot(&self, id: TensorDotId) -> TensorDotDimensions {
        self.side_table().tensor_dot(self.sections(), id)
    }

    /// Return one pooled tensor convolution dimension descriptor.
    #[inline(always)]
    pub(crate) fn tensor_convolution(
        &self,
        id: TensorConvolutionId,
    ) -> TensorConvolutionDimensions {
        self.side_table().tensor_convolution(self.sections(), id)
    }

    /// Return one pooled tensor convolution window descriptor.
    #[inline(always)]
    pub(crate) fn tensor_window(&self, id: TensorWindowId) -> TensorConvolutionWindow {
        self.side_table().tensor_window(self.sections(), id)
    }

    /// Return one pooled tensor gather descriptor.
    #[inline(always)]
    pub(crate) fn tensor_gather(&self, id: TensorGatherId) -> TensorGatherDimensions {
        self.side_table().tensor_gather(self.sections(), id)
    }

    /// Return one pooled tensor scatter descriptor.
    #[inline(always)]
    pub(crate) fn tensor_scatter(&self, id: TensorScatterId) -> TensorScatterDimensions {
        self.side_table().tensor_scatter(self.sections(), id)
    }

    /// Return one pooled callable signature.
    #[inline(always)]
    pub(crate) fn signature(&self, id: SignatureId) -> program::FunctionSignature {
        self.side_table().signature(self.sections(), id)
    }

    /// Borrow one pooled callable signature's parameters.
    #[inline(always)]
    pub(crate) fn signature_parameters(
        &self,
        signature: program::FunctionSignature,
    ) -> &[program::TypeId] {
        self.side_table()
            .signature_parameters(self.sections(), signature)
    }

    /// Borrow one pooled constant's bytes.
    #[inline(always)]
    pub(crate) fn constant_bytes(&self, id: ConstValueId) -> &[u8] {
        let constant = self.side_table().constant(self.sections(), id);

        self.side_table().constant_bytes(self.sections(), constant)
    }
}
