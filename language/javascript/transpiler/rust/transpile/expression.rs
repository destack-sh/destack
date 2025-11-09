use dyst_dir::{self as dir, Expression};

use crate::{Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    pub fn transpile_expression(
        &self,
        module: &'a dir::Module,
        expression: &'a dir::Expression,
        unit: &mut TranspilerUnit,
    ) {
        todo!("transpile expression");
    }
}
