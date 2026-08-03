use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Answer, CheckState, CheckWarning};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Reject duplicate extension members once the module settles.
    pub(in crate::check) fn report_extension_collisions(&mut self) -> CompilerResult<()> {
        let module = self.module_id;
        match self.body().check_duplicate_extension_members(module)? {
            Answer::Ready(_) => Ok(()),
            Answer::Pending(blockers) => Err(CompilerError::Internal {
                message: format!("extension collisions pended after solving: {blockers:?}"),
            }),
        }
    }

    /// Report runtime conditions whose checked type is one boolean literal.
    pub(in crate::check) fn report_constant_conditions(&mut self) -> CompilerResult<()> {
        // select the module when this check infers its bodies
        let modules: Vec<ModuleId> = [self.module_id]
            .into_iter()
            .filter(|module| self.infers_module(*module))
            .collect();

        // report each checked module
        for module in modules {
            self.report_module_constant_conditions(module)?;
        }

        Ok(())
    }

    /// Report runtime conditions in one module whose checked type is a boolean literal.
    fn report_module_constant_conditions(&mut self, module: ModuleId) -> CompilerResult<()> {
        let conditions = self.condition_expressions(module);

        // settle each checked condition before inspecting its canonical type
        for condition in conditions {
            let global = condition.into_global_any(module);
            let ty = self.require_node_type(global)?;
            let ty = self.settled_root(ty)?;
            let dir::Type::Literal(dir::ScalarLiteral::Boolean(value)) = self.ty(ty)? else {
                continue;
            };

            let anchor = self.diagnostic_anchor(module, condition.into_any());
            let warning = CheckWarning::ConstantCondition {
                anchor,
                module,
                value,
            };
            self.module_mut(module).warnings.push(warning.into());
        }

        Ok(())
    }

    /// Return every expression used as a runtime condition in one module.
    fn condition_expressions(&self, module: ModuleId) -> Vec<dir::LocalNodeId<dir::Expression>> {
        let view = self.module(module).view();
        let mut conditions = FxIndexSet::default();

        // collect condition sites directly from visible checked DIR
        for expression in view.iter_nodes::<dir::Expression>() {
            match view.get(expression) {
                dir::Expression::If { condition, .. } => {
                    for operand in &condition.operands {
                        if let dir::ConditionOperand::Expression { condition } = operand {
                            conditions.insert(*condition);
                        }
                    }
                }
                dir::Expression::While { condition, .. } => {
                    conditions.insert(*condition);
                }
                dir::Expression::For {
                    condition: Some(condition),
                    ..
                } => {
                    conditions.insert(*condition);
                }
                dir::Expression::Match { arms, .. } => {
                    for arm in arms {
                        if let Some(guard) = view.get(*arm).guard() {
                            conditions.insert(guard);
                        }
                    }
                }
                _ => {}
            }
        }

        conditions.into_iter().collect()
    }
}
