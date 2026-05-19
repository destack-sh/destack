use destack_dir::{Expression, LocalNodeId};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Reify selected member or call resolutions into explicit type checks.
    pub(super) fn reify_resolution(
        &self,
        _state: &mut ElaborateState<'_>,
        _expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        Ok(())
    }
}
