#![allow(clippy::type_complexity)]

use dyst_ast::{Keyword, NodeId, NodeType, Property, TokenType};

use crate::{Parser, ParserMark, ParserResult};

/// The keywords that can appear before a binding.
pub static BINDING_MODIFIERS: [Keyword; 6] = [
    Keyword::Static,
    Keyword::Override,
    Keyword::Readonly,
    Keyword::Public,
    Keyword::Protected,
    Keyword::Private,
];

impl<'a> Parser<'a> {
    /// Try to eat a property (return Property::Error if error and recovery is possible).
    #[inline]
    pub fn try_eat_property(&mut self, recover: TokenType) -> ParserResult<NodeId<Property>> {
        match self.eat_property() {
            Ok(property_id) => Ok(property_id),
            Err(err) => {
                let err = err.for_node_type(NodeType::Expression);
                let span = err.leaf_span();
                let start = ParserMark::new(span.start as usize);
                self.try_recover(start, recover, Some(err.clone()))?;
                Err(err)
            }
        }
    }

    /// Eat a property.
    pub fn eat_property(&mut self) -> ParserResult<NodeId<Property>> {
        // spread property
        if self.peek_token(TokenType::Spread).is_ok() {
            let start = self.mark();
            self.bump(); // eat spread
            let value = self.eat_expression()?;
            let property = Property::Spread { modifiers: None, value };
            return Ok(self.tree.insert(property, self.get_span_from(start)));
        }

        // modifiers prefix
        let modifiers = self.eat_binding_modifiers_prefix_maybe()?;

        // key
        let key = self.eat_key_maybe()?;

        // modifiers postfix
        let modifiers = self.eat_binding_modifiers_postfix_maybe(modifiers)?;

        // field type / value
        if self.options.in_variant || self.options.in_type {
            let start = self.mark();
            // type
            let ty = if self.peek_colon().is_ok() {
                self.bump(); // eat colon
                Some(self.eat_expression()?)
            } else {
                None
            };

            // value
            let value = if self.peek_token(TokenType::Assign).is_ok() {
                self.bump(); // eat assign
                Some(self.eat_expression()?)
            } else {
                None
            };

            // property
            let property = Property::Field {
                modifiers,
                key,
                ty,
                value,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        } else {
            let start = self.mark();
            // value
            let value = if self.peek_colon().is_ok() {
                self.bump(); // eat colon
                Some(self.eat_expression()?)
            } else {
                None
            };

            // property
            let property = Property::Field {
                modifiers,
                key,
                ty: None,
                value,
            };
            Ok(self.tree.insert(property, self.get_span_from(start)))
        }
    }

    /// Eat a variant body (without the header or `{` and `}`).
    pub fn eat_properties(&mut self) -> ParserResult<Vec<NodeId<Property>>> {
        // eat everything
        let mut properties: Vec<NodeId<Property>> = Vec::new();
        loop {
            // stop on closing brace
            if self.peek_token(TokenType::CloseBrace).is_ok() {
                break;
            }
            // consume any stop
            else if self.peek_any_stop().is_ok() {
                self.eat_any_stop_with_newlines()?;
                continue;
            }
            // keep eating properties
            else {
                match self.try_eat_property(TokenType::Newline) {
                    Ok(property_id) => {
                        properties.push(property_id);
                    }
                    Err(_) => continue, // keep eating other properties
                }
            }
        }
        Ok(properties)
    }
}

#[cfg(test)]
mod tests {
    use dyst_ast::{Expression, Key, Name, Property, TypeLiteral, Visibility};
    use dyst_source::{LanguageCompatibility, LanguageOptions};

    use crate::tests::TestParser;
    use crate::{assert_node, assert_string};

    #[test]
    fn test_parse_property_with_es_visibility_modifier() {
        let mut test = TestParser::new_with_options(
            r#"#name: string"#,
            LanguageOptions::default().with_compatibility(LanguageCompatibility::TypeScript),
        );
        let mut parser = test.prepare();

        let property = parser.eat_property().unwrap();
        assert_node!(parser.tree, property, Property::Field { modifiers: Some(modifiers), key: Some(Key::Name(Name::Identifier(name))), ty: Some(ty), value: None, .. } => {
            // #name means private for #Compatibility
            assert_eq!(modifiers.visibility.unwrap(), Visibility::Private);
            // name
            assert_string!(parser, *name, "name");
            // string
            assert_node!(parser.tree, *ty, Expression::TypeLiteral(TypeLiteral::String));
        });
    }
}
