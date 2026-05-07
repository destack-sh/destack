use super::lightning;
use super::lower::Lowerer;
use crate::{
    ComponentValue, ComponentValueList, Dimension, Function, ImportResource, Number, Symbol, Token,
    UrlResource,
};

impl<'a> Lowerer<'a> {
    /// Lower one raw component value source into one owned component list.
    pub(crate) fn lower_component_value_list_source(&mut self, source: &str) -> ComponentValueList {
        self.parse_component_value_list_source(source)
    }

    /// Lower one Lightning token list into one owned component list.
    pub(crate) fn lower_component_value_token_list(
        &mut self,
        tokens: &lightning::TokenList<'_>,
    ) -> ComponentValueList {
        let mut values = Vec::new();
        for token in &tokens.0 {
            self.lower_component_value_token_or_value(&mut values, token);
        }

        ComponentValueList { values }
    }

    /// Lower one Lightning token or value into owned component values.
    pub(crate) fn lower_component_value_token_or_value(
        &mut self,
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
            lightning::TokenOrValue::Color(value) => self.lower_color_value(values, value),
            lightning::TokenOrValue::Length(value) => values.push(self.lower_length_value(value)),
            lightning::TokenOrValue::Angle(value) => values.push(self.lower_angle_value(value)),
            lightning::TokenOrValue::Time(value) => values.push(self.lower_time_value(value)),
            lightning::TokenOrValue::Resolution(value) => {
                values.push(self.lower_resolution_value(value));
            }
            lightning::TokenOrValue::DashedIdent(value) => {
                values.push(self.lower_dashed_ident(value));
            }
            lightning::TokenOrValue::AnimationName(value) => {
                values.push(self.lower_animation_name(value));
            }
            lightning::TokenOrValue::UnresolvedColor(value) => {
                values.extend(self.lower_unresolved_color(value).values);
            }
        }
    }

    /// Lower one printable Lightning color into owned component values.
    fn lower_color_value(&mut self, values: &mut Vec<ComponentValue>, value: &lightning::CssColor) {
        let components = self.lower_serialized_component_value_list(value);

        values.extend(components.values);
    }

    /// Lower one dashed ident into one owned component value.
    fn lower_dashed_ident(&mut self, value: &lightning::DashedIdent<'_>) -> ComponentValue {
        ComponentValue::Token(Token::Ident(self.intern_string(value.0.as_ref())))
    }

    /// Lower one animation name into one owned component value.
    fn lower_animation_name(&mut self, value: &lightning::AnimationName<'_>) -> ComponentValue {
        match value {
            lightning::AnimationName::None => {
                ComponentValue::Token(Token::Ident(self.intern_string("none")))
            }
            lightning::AnimationName::Ident(value) => {
                ComponentValue::Token(Token::Ident(self.intern_string(value.0.as_ref())))
            }
            lightning::AnimationName::String(value) => {
                if Self::animation_name_string_is_identifier(value.as_ref()) {
                    ComponentValue::Token(Token::Ident(self.intern_string(value.as_ref())))
                } else {
                    ComponentValue::Token(Token::String(value.as_ref().to_string()))
                }
            }
        }
    }

    /// Return whether one animation string serializes as one identifier.
    fn animation_name_string_is_identifier(value: &str) -> bool {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "none" | "initial" | "inherit" | "unset" | "default" | "revert" | "revert-layer"
        )
    }

    /// Lower one Lightning length into one owned component value.
    pub(crate) fn lower_length_value(&mut self, value: &lightning::LengthValue) -> ComponentValue {
        let (value, unit) = value.to_unit_value();

        self.lower_dimension_value(value, unit)
    }

    /// Lower one Lightning length into one owned component value list.
    pub(crate) fn lower_length(&mut self, value: &lightning::Length) -> ComponentValueList {
        match value {
            lightning::Length::Value(value) => ComponentValueList {
                values: vec![self.lower_length_value(value)],
            },
            lightning::Length::Calc(_) => self.lower_serialized_component_value_list(value),
        }
    }

    /// Lower one Lightning percentage into one owned component value.
    pub(crate) fn lower_percentage_value(&self, value: &lightning::Percentage) -> ComponentValue {
        ComponentValue::Token(Token::Percentage(self.lower_percentage_number(value.0)))
    }

    /// Lower one Lightning length percentage into one owned component value list.
    pub(crate) fn lower_length_percentage(
        &mut self,
        value: &lightning::LengthPercentage,
    ) -> ComponentValueList {
        match value {
            lightning::DimensionPercentage::Dimension(value) => ComponentValueList {
                values: vec![self.lower_length_value(value)],
            },
            lightning::DimensionPercentage::Percentage(value) => ComponentValueList {
                values: vec![self.lower_percentage_value(value)],
            },
            lightning::DimensionPercentage::Calc(_) => {
                self.lower_serialized_component_value_list(value)
            }
        }
    }

    /// Lower one Lightning angle into one owned component value.
    pub(crate) fn lower_angle_value(&mut self, value: &lightning::Angle) -> ComponentValue {
        let (value, unit) = match value {
            lightning::Angle::Deg(value) => (*value, "deg"),
            lightning::Angle::Grad(value) => (*value, "grad"),
            lightning::Angle::Rad(value) => (*value, "rad"),
            lightning::Angle::Turn(value) => (*value, "turn"),
        };

        self.lower_dimension_value(value, unit)
    }

    /// Lower one Lightning time into one owned component value.
    pub(crate) fn lower_time_value(&mut self, value: &lightning::Time) -> ComponentValue {
        let (value, unit) = match value {
            lightning::Time::Seconds(value) => (*value, "s"),
            lightning::Time::Milliseconds(value) => (*value, "ms"),
        };

        self.lower_dimension_value(value, unit)
    }

    /// Lower one Lightning resolution into one owned component value.
    pub(crate) fn lower_resolution_value(
        &mut self,
        value: &lightning::Resolution,
    ) -> ComponentValue {
        let (value, unit) = match value {
            lightning::Resolution::Dpi(value) => (*value, "dpi"),
            lightning::Resolution::Dpcm(value) => (*value, "dpcm"),
            lightning::Resolution::Dppx(value) => (*value, "dppx"),
        };

        self.lower_dimension_value(value, unit)
    }

    /// Lower one numeric dimension into one owned component value.
    fn lower_dimension_value(&mut self, value: f32, unit: &str) -> ComponentValue {
        ComponentValue::Token(Token::Dimension(Dimension {
            number: Number {
                has_sign: value < 0.0,
                value,
                integer_value: (value.fract() == 0.0).then_some(value as i32),
            },
            unit: self.intern_string(unit),
        }))
    }

    /// Lower one parsed syntax component into one owned component value list.
    pub(crate) fn lower_parsed_component(
        &mut self,
        value: &lightning::ParsedComponent<'_>,
    ) -> ComponentValueList {
        match value {
            lightning::ParsedComponent::Length(value) => self.lower_length(value),
            lightning::ParsedComponent::Number(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Number(Number {
                    has_sign: value.is_sign_negative(),
                    value: *value,
                    integer_value: (value.fract() == 0.0).then_some(*value as i32),
                }))],
            },
            lightning::ParsedComponent::Percentage(value) => ComponentValueList {
                values: vec![self.lower_percentage_value(value)],
            },
            lightning::ParsedComponent::LengthPercentage(value) => {
                self.lower_length_percentage(value)
            }
            lightning::ParsedComponent::String(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::String(
                    value.as_ref().to_string(),
                ))],
            },
            lightning::ParsedComponent::Color(value) => {
                self.lower_serialized_component_value_list(value)
            }
            lightning::ParsedComponent::Image(value) => self.lower_image_value(value),
            lightning::ParsedComponent::Url(value) => ComponentValueList {
                values: vec![ComponentValue::Function(
                    self.lower_url_function(value.url.as_ref()),
                )],
            },
            lightning::ParsedComponent::Integer(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Number(Number {
                    has_sign: *value < 0,
                    value: *value as f32,
                    integer_value: Some(*value),
                }))],
            },
            lightning::ParsedComponent::Angle(value) => ComponentValueList {
                values: vec![self.lower_angle_value(value)],
            },
            lightning::ParsedComponent::Time(value) => ComponentValueList {
                values: vec![self.lower_time_value(value)],
            },
            lightning::ParsedComponent::Resolution(value) => ComponentValueList {
                values: vec![self.lower_resolution_value(value)],
            },
            lightning::ParsedComponent::TransformFunction(value) => ComponentValueList {
                values: vec![self.lower_transform_function_value(value)],
            },
            lightning::ParsedComponent::TransformList(value) => self.lower_transform_list(value),
            lightning::ParsedComponent::CustomIdent(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string(value.0.as_ref()),
                ))],
            },
            lightning::ParsedComponent::Literal(value) => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string(value.0.as_ref()),
                ))],
            },
            lightning::ParsedComponent::Repeated {
                components,
                multiplier,
            } => self.lower_repeated_parsed_components(components, multiplier),
            lightning::ParsedComponent::TokenList(value) => {
                self.lower_component_value_token_list(value)
            }
        }
    }

    /// Lower one printable Lightning value into one owned component value list.
    pub(crate) fn lower_serialized_component_value_list<T: lightning::ToCss>(
        &mut self,
        value: &T,
    ) -> ComponentValueList {
        self.lower_component_value_list_source(&self.serialize_value(value))
    }

    /// Lower one number or percentage into one owned component value.
    fn lower_number_or_percentage(&self, value: &lightning::NumberOrPercentage) -> ComponentValue {
        match value {
            lightning::NumberOrPercentage::Number(value) => {
                ComponentValue::Token(Token::Number(Number {
                    has_sign: value.is_sign_negative(),
                    value: *value,
                    integer_value: (value.fract() == 0.0).then_some(*value as i32),
                }))
            }
            lightning::NumberOrPercentage::Percentage(value) => self.lower_percentage_value(value),
        }
    }

    /// Lower one transform function into one owned component value.
    fn lower_transform_function_value(&mut self, value: &lightning::Transform) -> ComponentValue {
        let (name, arguments) = match value {
            lightning::Transform::Translate(x, y) => (
                "translate",
                vec![
                    self.lower_length_percentage(x).values,
                    self.lower_length_percentage(y).values,
                ],
            ),
            lightning::Transform::TranslateX(value) => (
                "translateX",
                vec![self.lower_length_percentage(value).values],
            ),
            lightning::Transform::TranslateY(value) => (
                "translateY",
                vec![self.lower_length_percentage(value).values],
            ),
            lightning::Transform::TranslateZ(value) => {
                ("translateZ", vec![self.lower_length(value).values])
            }
            lightning::Transform::Translate3d(x, y, z) => (
                "translate3d",
                vec![
                    self.lower_length_percentage(x).values,
                    self.lower_length_percentage(y).values,
                    self.lower_length(z).values,
                ],
            ),
            lightning::Transform::Scale(x, y) => (
                "scale",
                vec![
                    vec![self.lower_number_or_percentage(x)],
                    vec![self.lower_number_or_percentage(y)],
                ],
            ),
            lightning::Transform::ScaleX(value) => {
                ("scaleX", vec![vec![self.lower_number_or_percentage(value)]])
            }
            lightning::Transform::ScaleY(value) => {
                ("scaleY", vec![vec![self.lower_number_or_percentage(value)]])
            }
            lightning::Transform::ScaleZ(value) => {
                ("scaleZ", vec![vec![self.lower_number_or_percentage(value)]])
            }
            lightning::Transform::Scale3d(x, y, z) => (
                "scale3d",
                vec![
                    vec![self.lower_number_or_percentage(x)],
                    vec![self.lower_number_or_percentage(y)],
                    vec![self.lower_number_or_percentage(z)],
                ],
            ),
            lightning::Transform::Rotate(value) => {
                ("rotate", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::RotateX(value) => {
                ("rotateX", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::RotateY(value) => {
                ("rotateY", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::RotateZ(value) => {
                ("rotateZ", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::Rotate3d(x, y, z, angle) => (
                "rotate3d",
                vec![
                    vec![self.lower_float_number(*x)],
                    vec![self.lower_float_number(*y)],
                    vec![self.lower_float_number(*z)],
                    vec![self.lower_angle_value(angle)],
                ],
            ),
            lightning::Transform::Skew(x, y) => (
                "skew",
                vec![
                    vec![self.lower_angle_value(x)],
                    vec![self.lower_angle_value(y)],
                ],
            ),
            lightning::Transform::SkewX(value) => {
                ("skewX", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::SkewY(value) => {
                ("skewY", vec![vec![self.lower_angle_value(value)]])
            }
            lightning::Transform::Perspective(value) => {
                ("perspective", vec![self.lower_length(value).values])
            }
            lightning::Transform::Matrix(value) => (
                "matrix",
                vec![
                    vec![self.lower_float_number(value.a)],
                    vec![self.lower_float_number(value.b)],
                    vec![self.lower_float_number(value.c)],
                    vec![self.lower_float_number(value.d)],
                    vec![self.lower_float_number(value.e)],
                    vec![self.lower_float_number(value.f)],
                ],
            ),
            lightning::Transform::Matrix3d(value) => (
                "matrix3d",
                vec![
                    vec![self.lower_float_number(value.m11)],
                    vec![self.lower_float_number(value.m12)],
                    vec![self.lower_float_number(value.m13)],
                    vec![self.lower_float_number(value.m14)],
                    vec![self.lower_float_number(value.m21)],
                    vec![self.lower_float_number(value.m22)],
                    vec![self.lower_float_number(value.m23)],
                    vec![self.lower_float_number(value.m24)],
                    vec![self.lower_float_number(value.m31)],
                    vec![self.lower_float_number(value.m32)],
                    vec![self.lower_float_number(value.m33)],
                    vec![self.lower_float_number(value.m34)],
                    vec![self.lower_float_number(value.m41)],
                    vec![self.lower_float_number(value.m42)],
                    vec![self.lower_float_number(value.m43)],
                    vec![self.lower_float_number(value.m44)],
                ],
            ),
        };

        let arguments = Self::join_component_lists(arguments);

        ComponentValue::Function(Function {
            name: self.intern_string(name),
            url_resource: None,
            arguments: ComponentValueList { values: arguments },
        })
    }

    /// Lower one transform list into one owned component value list.
    fn lower_transform_list(&mut self, values: &lightning::TransformList) -> ComponentValueList {
        if values.0.is_empty() {
            return ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string("none"),
                ))],
            };
        }

        let mut lowered = Vec::new();

        for (index, value) in values.0.iter().enumerate() {
            if index > 0 {
                lowered.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            lowered.push(self.lower_transform_function_value(value));
        }

        ComponentValueList { values: lowered }
    }

    /// Lower one image value into one owned component value list.
    fn lower_image_value(&mut self, value: &lightning::Image<'_>) -> ComponentValueList {
        match value {
            lightning::Image::None => ComponentValueList {
                values: vec![ComponentValue::Token(Token::Ident(
                    self.intern_string("none"),
                ))],
            },
            lightning::Image::Url(value) => ComponentValueList {
                values: vec![ComponentValue::Function(
                    self.lower_url_function(value.url.as_ref()),
                )],
            },
            lightning::Image::Gradient(_) | lightning::Image::ImageSet(_) => {
                self.lower_serialized_component_value_list(value)
            }
        }
    }

    /// Lower one float number into one owned component value.
    fn lower_float_number(&self, value: f32) -> ComponentValue {
        ComponentValue::Token(Token::Number(Number {
            has_sign: value.is_sign_negative(),
            value,
            integer_value: (value.fract() == 0.0).then_some(value as i32),
        }))
    }

    /// Join multiple component lists with `, ` separators.
    fn join_component_lists(mut values: Vec<Vec<ComponentValue>>) -> Vec<ComponentValue> {
        let mut joined = Vec::new();

        for (index, value) in values.drain(..).enumerate() {
            if index > 0 {
                joined.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                joined.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            }

            joined.extend(value);
        }

        joined
    }

    /// Lower one repeated parsed component group into one owned component value list.
    fn lower_repeated_parsed_components(
        &mut self,
        components: &[lightning::ParsedComponent<'_>],
        multiplier: &lightning::SyntaxMultiplier,
    ) -> ComponentValueList {
        let mut values = Vec::new();

        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                match multiplier {
                    lightning::SyntaxMultiplier::None | lightning::SyntaxMultiplier::Space => {
                        values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                    }
                    lightning::SyntaxMultiplier::Comma => {
                        values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                        values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                    }
                }
            }

            values.extend(self.lower_parsed_component(component).values);
        }

        ComponentValueList { values }
    }

    /// Lower one Lightning custom function.
    pub(crate) fn lower_component_function(
        &mut self,
        function: &lightning::CustomFunction<'_>,
    ) -> Function {
        let arguments = self.lower_component_value_token_list(&function.arguments);
        let url_resource =
            (function.name.as_ref() == "url").then(|| self.build_function_url_resource(&arguments));

        Function {
            name: self.intern_string(function.name.as_ref()),
            url_resource,
            arguments,
        }
    }

    /// Lower one Lightning `url(...)` payload into one owned function.
    pub(crate) fn lower_url_function(&mut self, url: &str) -> Function {
        Function {
            name: self.intern_string("url"),
            url_resource: Some(self.build_url_resource(url)),
            arguments: ComponentValueList {
                values: vec![ComponentValue::Token(Token::String(url.to_string()))],
            },
        }
    }

    /// Lower one Lightning `var(...)` payload into one owned function.
    pub(crate) fn lower_variable_function(
        &mut self,
        variable: &lightning::Variable<'_>,
    ) -> Function {
        let mut values = vec![ComponentValue::Token(Token::Ident(
            self.intern_string(variable.name.ident.as_ref()),
        ))];

        if let Some(fallback) = &variable.fallback {
            values.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
            values.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
            values.extend(self.lower_component_value_token_list(fallback).values);
        }

        Function {
            name: self.intern_string("var"),
            url_resource: None,
            arguments: ComponentValueList { values },
        }
    }

    /// Lower one Lightning `env(...)` payload into one owned function.
    pub(crate) fn lower_environment_variable_function(
        &mut self,
        variable: &lightning::EnvironmentVariable<'_>,
    ) -> Function {
        let variable_name = self.environment_variable_name_source(&variable.name);
        let mut values = vec![ComponentValue::Token(Token::Ident(
            self.intern_string(&variable_name),
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
            name: self.intern_string("env"),
            url_resource: None,
            arguments: ComponentValueList { values },
        }
    }

    /// Lower one Lightning custom token into one owned token.
    pub(crate) fn lower_custom_token(&mut self, token: &lightning::CustomToken<'_>) -> Token {
        match token {
            lightning::CustomToken::Ident(value) => Token::Ident(self.intern_string(value)),
            lightning::CustomToken::AtKeyword(value) => Token::AtKeyword(self.intern_string(value)),
            lightning::CustomToken::Hash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: false,
            },
            lightning::CustomToken::IDHash(value) => Token::Hash {
                value: value.to_string(),
                is_identifier: true,
            },
            lightning::CustomToken::String(value) => Token::String(value.to_string()),
            lightning::CustomToken::UnquotedUrl(value) => Token::UnquotedUrl {
                value: value.to_string(),
                url_resource: Some(self.build_url_resource(value)),
            },
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
                unit: self.intern_string(unit),
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
        &mut self,
        name: &lightning::EnvironmentVariableName<'_>,
    ) -> String {
        self.serialize_value(name)
    }

    /// Lower one unresolved color into component values.
    pub(crate) fn lower_unresolved_color(
        &mut self,
        color: &lightning::UnresolvedColor<'_>,
    ) -> ComponentValueList {
        match color {
            lightning::UnresolvedColor::RGB { r, g, b, alpha } => {
                let alpha_values = self.lower_component_value_token_list(alpha);
                let arguments = vec![
                    ComponentValue::Token(Token::Number(Number {
                        has_sign: false,
                        value: *r,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Number(Number {
                        has_sign: false,
                        value: *g,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Number(Number {
                        has_sign: false,
                        value: *b,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Delimiter('/')),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                ];

                let mut arguments = arguments;
                arguments.extend(alpha_values.values);

                ComponentValueList {
                    values: vec![ComponentValue::Function(Function {
                        name: self.intern_string("rgb"),
                        url_resource: None,
                        arguments: ComponentValueList { values: arguments },
                    })],
                }
            }
            lightning::UnresolvedColor::HSL { h, s, l, alpha } => {
                let alpha_values = self.lower_component_value_token_list(alpha);
                let mut arguments = vec![
                    ComponentValue::Token(Token::Number(Number {
                        has_sign: h.is_sign_negative(),
                        value: *h,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Percentage(Number {
                        has_sign: s.is_sign_negative(),
                        value: *s / 100.0,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Percentage(Number {
                        has_sign: l.is_sign_negative(),
                        value: *l / 100.0,
                        integer_value: None,
                    })),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                    ComponentValue::Token(Token::Delimiter('/')),
                    ComponentValue::Token(Token::WhiteSpace(" ".to_string())),
                ];
                arguments.extend(alpha_values.values);

                ComponentValueList {
                    values: vec![ComponentValue::Function(Function {
                        name: self.intern_string("hsl"),
                        url_resource: None,
                        arguments: ComponentValueList { values: arguments },
                    })],
                }
            }
            lightning::UnresolvedColor::LightDark { light, dark } => {
                let light_values = self.lower_component_value_token_list(light);
                let dark_values = self.lower_component_value_token_list(dark);

                let mut arguments = light_values.values;
                arguments.push(ComponentValue::Token(Token::Symbol(Symbol::Comma)));
                arguments.push(ComponentValue::Token(Token::WhiteSpace(" ".to_string())));
                arguments.extend(dark_values.values);

                ComponentValueList {
                    values: vec![ComponentValue::Function(Function {
                        name: self.intern_string("light-dark"),
                        url_resource: None,
                        arguments: ComponentValueList { values: arguments },
                    })],
                }
            }
        }
    }

    /// Build one import resource payload.
    pub(crate) fn build_import_resource(&mut self, specifier: &str) -> ImportResource {
        ImportResource::new(self.allocate_resource_id(), specifier)
    }

    /// Build one url resource payload.
    pub(crate) fn build_url_resource(&mut self, specifier: &str) -> UrlResource {
        UrlResource::new(self.allocate_resource_id(), specifier)
    }

    /// Build one `url(...)` resource payload from function arguments.
    pub(crate) fn build_function_url_resource(
        &mut self,
        arguments: &ComponentValueList,
    ) -> UrlResource {
        let specifier = arguments
            .values
            .iter()
            .find_map(|value| match value {
                ComponentValue::Token(Token::String(value))
                | ComponentValue::Token(Token::UnquotedUrl { value, .. }) => Some(value.as_str()),
                ComponentValue::Token(Token::WhiteSpace(_)) => None,
                _ => None,
            })
            .unwrap_or_default();

        self.build_url_resource(specifier)
    }
}
