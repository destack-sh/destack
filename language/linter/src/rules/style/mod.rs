mod catch_error_name;
mod consistent_extension_style;
mod consistent_type_definitions;
mod consistent_type_imports;
mod dot_notation;
mod eqeqeq;
mod filename_case;
mod no_else_return;
mod no_lonely_if;
mod no_nested_ternary;
mod no_unneeded_ternary;
mod no_var;
mod object_shorthand;
mod operator_assignment;
mod prefer_arrow_callback;
mod prefer_as_const;
mod prefer_expression;
mod prefer_implicit_return;
mod prefer_loop;
mod prefer_range_literal;
mod prefer_template;

use crate::{BoxedLintRule, boxed};

pub use catch_error_name::*;
pub use consistent_extension_style::*;
pub use consistent_type_definitions::*;
pub use consistent_type_imports::*;
pub use dot_notation::*;
pub use eqeqeq::*;
pub use filename_case::*;
pub use no_else_return::*;
pub use no_lonely_if::*;
pub use no_nested_ternary::*;
pub use no_unneeded_ternary::*;
pub use no_var::*;
pub use object_shorthand::*;
pub use operator_assignment::*;
pub use prefer_arrow_callback::*;
pub use prefer_as_const::*;
pub use prefer_expression::*;
pub use prefer_implicit_return::*;
pub use prefer_loop::*;
pub use prefer_range_literal::*;
pub use prefer_template::*;

/// Get all style rules.
pub fn rules() -> Vec<BoxedLintRule> {
    vec![
        boxed(CatchErrorName),
        boxed(ConsistentExtensionStyle),
        boxed(ConsistentTypeDefinitions),
        boxed(ConsistentTypeImports),
        boxed(DotNotation),
        boxed(Eqeqeq),
        boxed(FilenameCaseRule),
        boxed(NoElseReturn),
        boxed(NoLonelyIf),
        boxed(NoNestedTernary),
        boxed(NoUnneededTernary),
        boxed(NoVar),
        boxed(ObjectShorthand),
        boxed(OperatorAssignment),
        boxed(PreferArrowCallback),
        boxed(PreferAsConst),
        boxed(PreferExpression),
        boxed(PreferImplicitReturn),
        boxed(PreferLoop),
        boxed(PreferRangeLiteral),
        boxed(PreferTemplate),
    ]
}
