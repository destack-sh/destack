use dyst_fir::format::FormatResult;

use crate::r#let::FormatScopedMutability;
use crate::{DystFormatter, FormatNode, Function, Keyword, NodeId, Runtime, Visibility};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Function> for Function {
    fn format_node(
        &self,
        node_id: NodeId<Function>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // visibility
        if let Some(visibility) = self.visibility {
            let keyword = match visibility {
                Visibility::Public => Keyword::Public,
                Visibility::Private => Keyword::Private,
            };
            write!(f, [keyword, space()])?;
        }

        // keyword
        write!(f, [Keyword::Function, space()])?;

        // name (with @)
        if self.runtime == Runtime::Static {
            write!(f, [token("@")])?;
        }
        if let Some(name) = self.name {
            write!(f, [name])?;
        }

        // static parameters
        if let Some(static_parameters) = &self.static_parameters
            && !static_parameters.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(static_parameters)
                        .finish()
                    })),
                    token(">")
                ])]
            )?;
        }

        // self parameter and dynamic parameters
        write!(
            f,
            [group(&format_args![
                token("("),
                soft_block_indent(&format_with(|f| {
                    // self parameter
                    let separator = format_with(|f| {
                        if_group_fits_on_line(&token(",")).format(f)?;
                        soft_line_break_or_space().format(f)
                    });
                    let mut join = f.join_with(&separator);
                    if let Some(self_parameter) = self.self_parameter.as_ref() {
                        let is_pointer = self_parameter.is_pointer;
                        let mutability = &self_parameter.mutability;
                        join.entry(&format_with(move |f| {
                            // pointer
                            if is_pointer {
                                write!(f, [token("&")])?;
                            }
                            // mutability
                            write!(
                                f,
                                [FormatScopedMutability::implicit_const(mutability.clone())]
                            )?;
                            if mutability.is_mutable() {
                                write!(f, [space()])?;
                            }
                            // self
                            write!(f, [Keyword::Self_])
                        }));
                    }
                    // dynamic parameters
                    join.entries(&self.dynamic_parameters);
                    join.finish()
                })),
                token(")")
            ])]
        )?;

        // return type
        if let Some(return_type) = self.return_type {
            write!(f, [space(), token("=>"), space(), return_type])?;
        }

        // with clause
        if let Some(with_id) = self.with {
            write!(f, [space(), with_id])?;
        }

        // body
        if let Some(body) = self.body {
            write!(f, [space(), body])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_function_simple() {
        assert_format!("function foo() {}", "function foo() { }", |p| p
            .eat_function(None));
    }

    #[test]
    fn test_format_function_with_parameters() {
        assert_format!(
            "function bar(x: int32, y: boolean) {}",
            "function bar(x: int32, y: boolean) { }",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_with_parameters_overflow() {
        assert_format!(
            "function bar(x: int32, y: boolean, z: string) {}",
            "function bar(\n\tx: int32\n\ty: boolean\n\tz: string\n) { }",
            |p| p.eat_function(None),
            DystFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_with_return_type() {
        assert_format!(
            "function baz() => int32 {}",
            "function baz() => int32 { }",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_with_static_parameters() {
        assert_format!(
            "function generic<T, U>() {}",
            "function generic<T, U>() { }",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_with_mutable_self_reference() {
        assert_format!(
            "function mutate(&var(x, y) self) {}",
            "function mutate(&var(x, y) self) { }",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_with_with_clause_overflow() {
        assert_format!(
            "function foo() with Time, Place, Something, Foo, Baz {}",
            "function foo() with (\n\tTime\n\tPlace\n\tSomething\n\tFoo\n\tBaz\n) { }",
            |p| p.eat_function(None),
            DystFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_function_declaration() {
        assert_format!(
            "function external() => int32",
            "function external() => int32",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_static_runtime() {
        assert_format!("function @comptime() {}", "function @comptime() { }", |p| p
            .eat_function(None));
    }

    #[test]
    fn test_format_function_with_with_and_return() {
        assert_format!(
            "function foo() => int32 with Disk {}",
            "function foo() => int32 with Disk { }",
            |p| p.eat_function(None)
        );
    }
}
