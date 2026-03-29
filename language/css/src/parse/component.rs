use super::lightning;
use super::lower::Lowerer;
use super::parse::Parser;
use crate::print::print_component_value_list;
use crate::{ComponentValue, ComponentValueList, Dimension, Function, Number, Symbol, Token};

impl<'a> Lowerer<'a> {
    /// Lower one raw component value source into one owned component list.
    pub(crate) fn lower_component_value_list_source(&self, source: &str) -> ComponentValueList {
        Parser::parse_component_value_list(source)
    }

    /// Lower one Lightning token list into one owned component list.
    pub(crate) fn lower_component_value_token_list(
        &self,
        tokens: &lightning::TokenList<'_>,
    ) -> ComponentValueList {
        let mut values = Vec::new();

        for token in &tokens.0 {
            self.lower_component_value_token_or_value(&mut values, token);
        }

        ComponentValueList { values }
    }

    /// Lower one printable Lightning CSS value into owned component values.
    pub(crate) fn lower_component_value_css<T: lightning::ToCss>(
        &self,
        values: &mut Vec<ComponentValue>,
        value: &T,
    ) {
        let source = self.serialize_value(value);
        let components = self.lower_component_value_list_source(&source);

        values.extend(components.values);
    }

    /// Lower one Lightning token or value into owned component values.
    pub(crate) fn lower_component_value_token_or_value(
        &self,
        values: &mut Vec<ComponentValue>,
        token: &lightning::TokenOrValue<'_>,
    ) {
        match token {
            lightning::TokenOrValue::Token(token) => {
                values.push(ComponentValue::Token(self.lower_custom_token(token)));
            }
            lightning::TokenOrValue::Function(function) => {
                values.push(ComponentValue::Function(
                    self.lower_component_function(function),
                ));
            }
            lightning::TokenOrValue::Url(url) => {
                values.push(ComponentValue::Function(self.lower_url_function(&url.url)));
            }
            lightning::TokenOrValue::Var(variable) => {
                values.push(ComponentValue::Function(
                    self.lower_variable_function(variable),
                ));
            }
            lightning::TokenOrValue::Env(variable) => {
                values.push(ComponentValue::Function(
                    self.lower_environment_variable_function(variable),
                ));
            }
            lightning::TokenOrValue::Color(value) => self.lower_component_value_css(values, value),
            lightning::TokenOrValue::Length(value) => self.lower_component_value_css(values, value),
            lightning::TokenOrValue::Angle(value) => self.lower_component_value_css(values, value),
            lightning::TokenOrValue::Time(value) => self.lower_component_value_css(values, value),
            lightning::TokenOrValue::Resolution(value) => {
                self.lower_component_value_css(values, value)
            }
            lightning::TokenOrValue::DashedIdent(value) => {
                self.lower_component_value_css(values, value)
            }
            lightning::TokenOrValue::AnimationName(value) => {
                self.lower_component_value_css(values, value)
            }
            lightning::TokenOrValue::UnresolvedColor(value) => {
                let source = self.unresolved_color_source(value);
                values.extend(self.lower_component_value_list_source(&source).values);
            }
        }
    }

    /// Lower one Lightning custom function.
    pub(crate) fn lower_component_function(
        &self,
        function: &lightning::CustomFunction<'_>,
    ) -> Function {
        Function {
            name: function.name.to_string(),
            arguments: self.lower_component_value_token_list(&function.arguments),
        }
    }

    /// Lower one Lightning `url(...)` payload into one owned function.
    pub(crate) fn lower_url_function(&self, url: &str) -> Function {
        Function {
            name: "url".to_string(),
            arguments: ComponentValueList {
                values: vec![ComponentValue::Token(Token::String(url.to_string()))],
            },
        }
    }

    /// Lower one Lightning `var(...)` payload into one owned function.
    pub(crate) fn lower_variable_function(&self, variable: &lightning::Variable<'_>) -> Function {
        let mut values = vec![ComponentValue::Token(Token::Ident(
            variable.name.ident.to_string(),
        ))];

        if let Some(fallback) = &variable.fallback {
            values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.extend(self.lower_component_value_token_list(fallback).values);
        }

        Function {
            name: "var".to_string(),
            arguments: ComponentValueList { values },
        }
    }

    /// Lower one Lightning `env(...)` payload into one owned function.
    pub(crate) fn lower_environment_variable_function(
        &self,
        variable: &lightning::EnvironmentVariable<'_>,
    ) -> Function {
        let mut values = vec![ComponentValue::Token(Token::Ident(
            self.environment_variable_name_source(&variable.name),
        ))];

        for index in &variable.indices {
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.push(ComponentValue::Token(Token::Number(Number {
                has_sign: *index < 0,
                value: *index as f32,
                integer_value: Some(*index),
            })));
        }

        if let Some(fallback) = &variable.fallback {
            values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.extend(self.lower_component_value_token_list(fallback).values);
        }

        Function {
            name: "env".to_string(),
            arguments: ComponentValueList { values },
        }
    }

    /// Lower one Lightning custom token into one owned token.
    pub(crate) fn lower_custom_token(&self, token: &lightning::CustomToken<'_>) -> Token {
        match token {
            lightning::CustomToken::Ident(value) => Token::Ident(value.to_string()),
            lightning::CustomToken::AtKeyword(value) => Token::AtKeyword(value.to_string()),
            lightning::CustomToken::Hash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: false,
            },
            lightning::CustomToken::IDHash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: true,
            },
            lightning::CustomToken::String(value) => Token::String(value.to_string()),
            lightning::CustomToken::UnquotedUrl(value) => Token::UnquotedUrl(value.to_string()),
            lightning::CustomToken::BadUrl(value) => Token::BadUrl(value.to_string()),
            lightning::CustomToken::BadString(value) => Token::BadString(value.to_string()),
            lightning::CustomToken::Delim(value) => Token::Delimiter(*value),
            lightning::CustomToken::Number {
                has_sign,
                value,
                int_value,
            } => Token::Number(Number {
                has_sign: *has_sign,
                value: *value,
                integer_value: *int_value,
            }),
            lightning::CustomToken::Percentage {
                has_sign,
                unit_value,
                int_value,
            } => Token::Percentage(Number {
                has_sign: *has_sign,
                value: *unit_value,
                integer_value: *int_value,
            }),
            lightning::CustomToken::Dimension {
                has_sign,
                value,
                int_value,
                unit,
            } => Token::Dimension(Dimension {
                number: Number {
                    has_sign: *has_sign,
                    value: *value,
                    integer_value: *int_value,
                },
                unit: unit.to_string(),
            }),
            lightning::CustomToken::WhiteSpace(value) => Token::WhiteSpace(value.to_string()),
            lightning::CustomToken::Comment(value) => Token::Comment(value.to_string()),
            lightning::CustomToken::Colon => Token::Symbol(Symbol::Colon),
            lightning::CustomToken::Semicolon => Token::Symbol(Symbol::Semicolon),
            lightning::CustomToken::Comma => Token::Symbol(Symbol::Comma),
            lightning::CustomToken::IncludeMatch => Token::Symbol(Symbol::IncludeMatch),
            lightning::CustomToken::DashMatch => Token::Symbol(Symbol::DashMatch),
            lightning::CustomToken::PrefixMatch => Token::Symbol(Symbol::PrefixMatch),
            lightning::CustomToken::SuffixMatch => Token::Symbol(Symbol::SuffixMatch),
            lightning::CustomToken::SubstringMatch => Token::Symbol(Symbol::SubstringMatch),
            lightning::CustomToken::CDO => Token::Symbol(Symbol::Cdo),
            lightning::CustomToken::CDC => Token::Symbol(Symbol::Cdc),
            lightning::CustomToken::Function(_)
            | lightning::CustomToken::ParenthesisBlock
            | lightning::CustomToken::SquareBracketBlock
            | lightning::CustomToken::CurlyBracketBlock
            | lightning::CustomToken::CloseParenthesis
            | lightning::CustomToken::CloseSquareBracket
            | lightning::CustomToken::CloseCurlyBracket => {
                panic!("nested lightning custom tokens must be lowered structurally")
            }
        }
    }

    /// Return one Lightning environment variable name as canonical CSS source.
    pub(crate) fn environment_variable_name_source(
        &self,
        name: &lightning::EnvironmentVariableName<'_>,
    ) -> String {
        self.serialize_value(name)
    }

    /// Return one unresolved color as canonical CSS source.
    pub(crate) fn unresolved_color_source(&self, color: &lightning::UnresolvedColor<'_>) -> String {
        match color {
            lightning::UnresolvedColor::RGB { r, g, b, alpha } => {
                let alpha =
                    print_component_value_list(&self.lower_component_value_token_list(alpha));
                format!("rgb({r} {g} {b} / {alpha})")
            }
            lightning::UnresolvedColor::HSL { h, s, l, alpha } => {
                let alpha =
                    print_component_value_list(&self.lower_component_value_token_list(alpha));
                format!("hsl({h} {s} {l} / {alpha})")
            }
            lightning::UnresolvedColor::LightDark { light, dark } => {
                let light =
                    print_component_value_list(&self.lower_component_value_token_list(light));
                let dark = print_component_value_list(&self.lower_component_value_token_list(dark));
                format!("light-dark({light}, {dark})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::print::print_component_value_list;
    use crate::{BlockKind, ComponentValue};

    use super::Parser;

    /// Preserve generic CSS component values through tokenization and printing.
    #[test]
    fn test_roundtrip_component_value_list_source() {
        let source = "@media screen and (width >= 20px){.button:hover{color:red;url(\"/a.png\")}}";
        let components = Parser::parse_component_value_list(source);

        assert_eq!(print_component_value_list(&components), source);
    }

    /// Preserve nested functions and blocks as structured component values.
    #[test]
    fn test_parse_nested_component_values() {
        let components = Parser::parse_component_value_list(
            "url(\"/a.png\") calc(100% - 1rem) [data-kind=\"x\"]",
        );

        assert!(matches!(
            components.values.first(),
            Some(ComponentValue::Function(function)) if function.name == "url"
        ));
        assert!(matches!(
            components
                .values
                .iter()
                .find(|value| matches!(value, ComponentValue::Function(function) if function.name == "calc")),
            Some(ComponentValue::Function(_))
        ));
        assert!(matches!(
            components
                .values
                .iter()
                .find(|value| matches!(value, ComponentValue::Block(block) if block.kind == BlockKind::SquareBracket)),
            Some(ComponentValue::Block(_))
        ));
    }
}
