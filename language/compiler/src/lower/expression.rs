//! Lower DIR expressions to MIR.

use destack_base::StringPool;
use destack_dir::{self as dir, Expression};
use destack_mir::{self as mir, GlobalInitializer, Type};
use destack_workspace::{Module, ModuleDir};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
#[allow(unused_variables)]
impl Compiler {
    /// Lower a single DIR expression to MIR.
    ///
    /// This handles top-level expressions that produce globals, functions, etc.
    /// Not all expressions produce MIR output (e.g., imports are resolved at link time).
    pub(super) fn lower_expression(
        &self,
        module: &Module,
        dir: &ModuleDir,
        dir_tree: &dir::NodeTree,
        expr: &Expression,
        mir_tree: &mut mir::NodeTree,
        mir_strings: &StringPool,
    ) {
        todo!("lower_expression");
    }

    /// Lower a declarator (single binding in a let/const statement) to a MIR global.
    pub(super) fn lower_declarator(
        &self,
        module: &Module,
        dir: &ModuleDir,
        dir_tree: &dir::NodeTree,
        declarator: &dir::Declarator,
        is_const: bool,
        is_exported: bool,
        mir_tree: &mut mir::NodeTree,
        mir_strings: &StringPool,
    ) {
        todo!("lower_declarator");
    }

    /// Lower a value expression to a global initializer.
    /// Returns the initializer and the MIR type.
    pub(super) fn lower_value_to_initializer(
        &self,
        module: &Module,
        _dir: &ModuleDir,
        _dir_tree: &dir::NodeTree,
        value: &Expression,
        mir_tree: &mut mir::NodeTree,
        _mir_strings: &StringPool,
    ) -> Option<(GlobalInitializer, mir::LocalNodeId<Type>)> {
        todo!("lower_value_to_initializer");
    }
}
