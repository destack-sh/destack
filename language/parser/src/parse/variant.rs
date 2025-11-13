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
        todo!("not implemented")
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

            todo!("nocheckin")
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
