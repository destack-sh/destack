use crate::{DystFormatter, FormatNode};
use dyst_ast::{EnumField, NodeId};
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, EnumField> for EnumField {
    fn format_node(
        &self,
        node_id: NodeId<EnumField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // name
        write!(f, [self.name])?;

        // value
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::TestFormatter;
    use crate::{DystFormatOptions, assert_format};
    use dyst_ast::DeclarationDescriptor;

    #[test]
    fn test_format_enum_empty() {
        assert_format!(
            "enum { }",
            "enum { }",
            |p| p.eat_enum(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_enum_with_simple_fields() {
        assert_format!(
            "enum { A, B }",
            "enum {\n\tA\n\tB\n}",
            |p| p.eat_enum(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_annotations() {
        assert_format!(
            "enum { A }",
            "enum {\n\tA\n}",
            |p| p.eat_enum(DeclarationDescriptor::default()),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_enum_with_static_parameters() {
        let source = r"enum Machine<T: int32 = 3, IsSomething: boolean = true> {
    A = 1
    B = T
    @if(IsSomething)
    C = 3
}";
        assert_format!(
            source,
            source,
            |p| p.eat_enum(DeclarationDescriptor::default()),
            DystFormatOptions::default()
        );
    }
}
