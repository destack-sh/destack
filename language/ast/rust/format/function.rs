use dyst_language_fir::format::FormatResult;

use crate::{
    DystFormatter, FormatNode, Function, FunctionStyle, Keyword, Mutability, NodeId, Runtime,
    Visibility,
};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Function> for Function {
    fn format_node(
        &self,
        _node_id: NodeId<Function>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        debug_assert!(self.style == FunctionStyle::Function);

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
        if let Some(static_parameters) = &self.static_parameters {
            if static_parameters.is_empty() {
                write!(f, [token("<>")])?;
            } else {
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
                        let is_mutable = self_parameter.mutability == Mutability::Mutable;
                        join.entry(&format_with(move |f| {
                            // pointer
                            if is_pointer {
                                write!(f, [token("&")])?;
                            }
                            // mutability
                            if is_mutable {
                                write!(f, [Keyword::Var, space()])?;
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

        // with clause
        if let Some(with_id) = self.with {
            write!(f, [space(), with_id])?;
        }

        // return type
        if let Some(return_type) = self.return_type {
            write!(f, [space(), token("=>"), space(), return_type])?;
        }

        // body
        if let Some(body) = self.body {
            write!(f, [space(), body])
        } else {
            Ok(())
        }
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
    fn test_format_function_with_mutable_self() {
        assert_format!(
            "function mutate(&var self) {}",
            "function mutate(&var self) { }",
            |p| p.eat_function(None)
        );
    }

    #[test]
    fn test_format_function_with_with_clause_overflow() {
        assert_format!(
            "function foo() with Time, Place, Something, Foo, Baz {}",
            "function foo() with (\n\t\tTime\n\t\tPlace\n\t\tSomething\n\t\tFoo\n\t\tBaz\n) { }",
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
            "function foo() with Disk => int32 {}",
            "function foo() with Disk => int32 { }",
            |p| p.eat_function(None)
        );
    }
}
