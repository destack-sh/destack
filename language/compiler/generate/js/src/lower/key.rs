use destack_core::StringId;
use destack_dir::is_identifier;
use smallvec::smallvec;
use {destack_dir as dir, destack_js as js};

use crate::{CodegenJsError, CodegenJsResult, CodegenJsResultExt, ModuleLowerer};

impl ModuleLowerer<'_> {
    /// Insert one path expression from string segments.
    fn insert_path_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        segments: &[&str],
    ) -> js::LocalNodeId<js::Expression> {
        let segments = segments
            .iter()
            .map(|segment| self.strings.intern(segment))
            .collect();
        let path = js::Path { segments };
        let expression = js::Expression::Path {
            path,
            generic_arguments: vec![],
        };

        self.tree
            .insert_from_source_any(expression, self.module.id, source_id)
    }

    /// Insert one string literal expression.
    fn insert_string_literal_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        value: StringId,
    ) -> js::LocalNodeId<js::Expression> {
        let value = value;
        let expression = js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(value),
        };

        self.tree
            .insert_from_source_any(expression, self.module.id, source_id)
    }

    /// Lower one symbol key into one computed expression key.
    fn lower_symbol_key_expression(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        key: dir::SymbolKey,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
        let expression_id = match key {
            dir::SymbolKey::Registry(name) => {
                let callee = self.insert_path_expression(source_id, &["Symbol", "for"]);
                let value = self.insert_string_literal_expression(source_id, name);
                let argument = js::Argument::Positional { value };
                let argument_id =
                    self.tree
                        .insert_from_source_any(argument, self.module.id, source_id);
                let call = js::Expression::Call {
                    position: js::PostfixPosition::Direct,
                    left: callee,
                    generic_arguments: vec![],
                    arguments: vec![argument_id],
                };
                self.tree
                    .insert_from_source_any(call, self.module.id, source_id)
            }
            dir::SymbolKey::Unique(symbol_id) => {
                if symbol_id.module_id != self.module.id {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "remote unique symbol keys need source-backed lowering in JS output"
                                .to_string(),
                        ),
                    });
                }

                let symbol = self.symbols.get_symbol(dir::LocalSymbolId::from(symbol_id));
                let Some(dir::StaticKey::Name(name)) = symbol.key else {
                    return Err(CodegenJsError::UnsupportedConstruct {
                        node: source_id.into_global(self.module.id),
                        message: Some(
                            "unique symbol keys need identifier-backed symbols in JS output"
                                .to_string(),
                        ),
                    });
                };

                let segment = name;
                let path = js::Path {
                    segments: smallvec![segment],
                };
                let expression = js::Expression::Path {
                    path,
                    generic_arguments: vec![],
                };
                let expression_id =
                    self.tree
                        .insert_from_source_any(expression, self.module.id, source_id);
                self.set_global_node_symbol(expression_id, symbol_id);
                expression_id
            }
        };

        Ok(expression_id)
    }

    /// Lower a string to a name.
    pub fn lower_string_to_name(&mut self, string_id: StringId) -> js::Name {
        let string = self.source_strings.get(string_id);
        let string_id = self.strings.intern(&string);

        if is_identifier(string.as_ref()) {
            js::Name::Identifier(string_id)
        } else {
            js::Name::String(string_id)
        }
    }

    /// Lower a name from DIR into JS AST.
    pub fn lower_name(&mut self, name: dir::Name) -> js::Name {
        let name_id = name.string();

        match name {
            dir::Name::Identifier(_) => js::Name::Identifier(name_id),
            dir::Name::String(_) => js::Name::String(name_id),
            dir::Name::Number(_) => js::Name::String(name_id),
        }
    }

    /// Lower a key from DIR into JS AST.
    pub fn lower_key(&mut self, key: dir::Key) -> CodegenJsResult<js::Key> {
        let key = match key {
            dir::Key::Name(name) => {
                let name = self.lower_name(name);
                js::Key::Name(name)
            }
            dir::Key::Private(name) => {
                let name = name;
                js::Key::Private(name)
            }
            dir::Key::Expression(expression_id) => {
                let expression_id = self
                    .lower_expression(expression_id)
                    .expect_node::<js::Expression>(
                        expression_id.into_global_any(self.module.id),
                        self,
                    )?;
                js::Key::Expression(expression_id)
            }
        };

        Ok(key)
    }

    /// Lower a static key from DIR into JS AST.
    pub fn lower_static_key(
        &mut self,
        source_id: dir::LocalNodeIdAny,
        key: dir::StaticKey,
    ) -> CodegenJsResult<js::Key> {
        let key = match key {
            dir::StaticKey::Name(name) | dir::StaticKey::Number(name) => {
                let name = self.lower_string_to_name(name);
                js::Key::Name(name)
            }
            dir::StaticKey::Symbol(symbol) => {
                let expression_id = self.lower_symbol_key_expression(source_id, symbol)?;
                js::Key::Expression(expression_id)
            }
        };

        Ok(key)
    }
}
