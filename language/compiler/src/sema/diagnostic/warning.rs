use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, CheckWarning};

impl CheckState<'_> {
    /// Report runtime conditions whose checked type is one boolean literal.
    pub(in crate::sema) fn report_constant_conditions(&mut self) -> CompilerResult<()> {
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

            // statically absent conditions never check and carry no type
            let Some(ty) = self.committed_node_type(global) else {
                continue;
            };
            let ty = self.shallow_resolve(ty)?;
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
        for expression in view.iter_node_ids_of_type::<dir::Expression>() {
            match view.get(expression) {
                dir::Expression::If { condition, .. }
                | dir::Expression::While { condition, .. } => {
                    for condition in condition.expressions() {
                        conditions.insert(condition);
                    }
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
