use destack_core::StringId;
use destack_source::Span;

use super::printer::Printer;
use crate::{
    AccessorKind, AssignOperator, BinaryOperator, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, DependencyBinding, JsPrintResult, Keyword, Mutability, Name, PrimitiveType,
    ScalarLiteral, TypeLiteral, UnaryOperator, VarianceModifier, Visibility,
};

impl<'a> Printer<'a> {
    /// Print one name.
    pub(crate) fn write_name(&mut self, name: Name) {
        match name {
            Name::Identifier(name) => self.write_string_id(name),
            Name::String(name) => self.write_string_literal(name),
        }
    }

    /// Print one name with one source span.
    pub(crate) fn write_name_with_source_span(&mut self, name: Name, source_span: Option<Span>) {
        match name {
            Name::Identifier(name) => self.write_identifier_with_source_span(name, source_span),
            Name::String(name) => self.write_string_literal_with_source_span(name, source_span),
        }
    }

    /// Print one dependency binding keyword.
    pub(crate) fn write_dependency_binding(&mut self, binding: DependencyBinding) {
        match binding {
            DependencyBinding::Named => self.write_keyword(Keyword::Export),
            DependencyBinding::Default => {
                self.write_keyword(Keyword::Export);
                self.write_keyword(Keyword::Default);
            }
            DependencyBinding::Namespace => self.write_keyword(Keyword::Export),
        }
    }

    /// Print one scalar literal.
    pub(crate) fn print_scalar_literal(&mut self, scalar: &ScalarLiteral) {
        match scalar {
            ScalarLiteral::Null => self.write_punct("null"),
            ScalarLiteral::Undefined => self.write_punct("undefined"),
            ScalarLiteral::Boolean(value) => {
                if *value {
                    self.write_punct("true");
                } else {
                    self.write_punct("false");
                }
            }
            ScalarLiteral::Bigint(value) => self.write_punct(&format!("{value}n")),
            ScalarLiteral::Number(value) => {
                let encoded = Self::encode_js_number_literal(*value);
                self.write_punct(&encoded);
            }
            ScalarLiteral::String(value) => self.write_string_literal(*value),
            ScalarLiteral::RegexString { content, flags } => {
                self.write_punct("/");
                self.write_string_id(*content);
                self.write_punct("/");

                if let Some(flags) = flags {
                    self.write_string_id(*flags);
                }
            }
        }
    }

    /// Print one template literal.
    pub(crate) fn print_template_literal(
        &mut self,
        template_literal: &crate::TemplateLiteral,
    ) -> JsPrintResult<()> {
        match template_literal {
            crate::TemplateLiteral::String { template } => {
                self.write_punct("`");
                self.write_string_id(*template);
                self.write_punct("`");
            }
            crate::TemplateLiteral::TaggedString { tag, template } => {
                self.print_path(tag);
                self.write_punct("`");
                self.write_string_id(*template);
                self.write_punct("`");
            }
            crate::TemplateLiteral::InterpolatedString {
                template,
                expressions,
            } => {
                self.write_punct("`");

                for (index, string) in template.iter().enumerate() {
                    self.write_string_id(*string);

                    if let Some(expression) = expressions.get(index) {
                        self.write_punct("${");
                        self.print_expression_id(*expression)?;
                        self.write_punct("}");
                    }
                }

                self.write_punct("`");
            }
            crate::TemplateLiteral::TaggedInterpolatedString {
                tag,
                template,
                expressions,
            } => {
                self.print_path(tag);
                self.write_punct("`");

                for (index, string) in template.iter().enumerate() {
                    self.write_string_id(*string);

                    if let Some(expression) = expressions.get(index) {
                        self.write_punct("${");
                        self.print_expression_id(*expression)?;
                        self.write_punct("}");
                    }
                }

                self.write_punct("`");
            }
        }

        Ok(())
    }

    /// Print one type literal.
    pub(crate) fn print_type_literal(&mut self, literal: &TypeLiteral) {
        match literal {
            TypeLiteral::Never => self.write_punct("never"),
            TypeLiteral::Any => self.write_punct("any"),
            TypeLiteral::Undefined => self.write_punct("undefined"),
            TypeLiteral::Unknown => self.write_punct("unknown"),
            TypeLiteral::Object => self.write_punct("object"),
            TypeLiteral::Void => self.write_punct("void"),
            TypeLiteral::Null => self.write_punct("null"),
            TypeLiteral::Primitive(primitive) => self.print_primitive_type(primitive),
            TypeLiteral::ScalarLiteral(scalar_literal) => self.print_scalar_literal(scalar_literal),
        }
    }

    /// Print one primitive type.
    pub(crate) fn print_primitive_type(&mut self, primitive: &PrimitiveType) {
        match primitive {
            PrimitiveType::Boolean => self.write_punct("boolean"),
            PrimitiveType::String => self.write_punct("string"),
            PrimitiveType::Bigint => self.write_punct("bigint"),
            PrimitiveType::Number => self.write_punct("number"),
            PrimitiveType::Symbol => self.write_punct("symbol"),
            PrimitiveType::UniqueSymbol => self.write_punct("unique symbol"),
        }
    }

    /// Print one binding modifier prefix.
    pub(crate) fn print_binding_modifiers_prefix(&mut self, modifiers: Option<BindingModifier>) {
        let Some(modifiers) = modifiers else {
            return;
        };

        if let Some(variance) = modifiers.variance {
            match variance {
                VarianceModifier::In => self.write_punct("in"),
                VarianceModifier::Out => self.write_punct("out"),
                VarianceModifier::InOut => {
                    self.write_punct("in");
                    self.write_punct("out");
                }
            }
        }

        if let Some(visibility) = modifiers.visibility {
            self.write_visibility(visibility);
        }

        if modifiers.anchor == Some(BindingAnchor::Static) {
            self.write_keyword(Keyword::Static);
        }

        if modifiers.mutability == Some(Mutability::Immutable) {
            self.write_keyword(Keyword::Readonly);
        }

        if modifiers.operator == Some(BindingOperator::AsConst) {
            self.write_keyword(Keyword::Const);
        }

        if modifiers.accessor == Some(AccessorKind::Accessor) {
            self.write_keyword(Keyword::Accessor);
        }
    }

    /// Print one binding modifier postfix.
    pub(crate) fn print_binding_modifiers_postfix(&mut self, modifiers: Option<BindingModifier>) {
        if modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe)) {
            self.write_punct("?");
        }

        if modifiers.is_some_and(|modifiers| modifiers.definite) {
            self.write_punct("!");
        }
    }

    /// Print one visibility keyword.
    pub(crate) fn write_visibility(&mut self, visibility: Visibility) {
        match visibility {
            Visibility::Public => self.write_keyword(Keyword::Public),
            Visibility::Protected => self.write_keyword(Keyword::Protected),
            Visibility::Private => self.write_keyword(Keyword::Private),
        }
    }

    /// Print one keyword.
    pub(crate) fn write_keyword(&mut self, keyword: Keyword) {
        self.write_punct(keyword.as_str());
    }

    /// Print one unary operator.
    pub(crate) fn write_unary_operator(&mut self, operator: UnaryOperator) {
        self.write_punct(match operator {
            UnaryOperator::PostIncrement => "++",
            UnaryOperator::PostDecrement => "--",
            UnaryOperator::PreIncrement => "++",
            UnaryOperator::PreDecrement => "--",
            UnaryOperator::Plus => "+",
            UnaryOperator::Negate => "-",
            UnaryOperator::ElementwiseNot => "~",
            UnaryOperator::Not => "!",
            UnaryOperator::Typeof => "typeof",
            UnaryOperator::Void => "void",
        });
    }

    /// Print one binary operator.
    pub(crate) fn write_binary_operator(&mut self, operator: BinaryOperator) {
        self.write_punct(match operator {
            BinaryOperator::Multiply => "*",
            BinaryOperator::Exponent => "**",
            BinaryOperator::Divide => "/",
            BinaryOperator::Remainder => "%",
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::ShiftLeft => "<<",
            BinaryOperator::ShiftRight => ">>",
            BinaryOperator::UnsignedShiftRight => ">>>",
            BinaryOperator::ElementwiseAnd => "&",
            BinaryOperator::ElementwiseXor => "^",
            BinaryOperator::ElementwiseOr => "|",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::EqualStrict => "===",
            BinaryOperator::NotEqualStrict => "!==",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::Coalesce => "??",
            BinaryOperator::In => "in",
            BinaryOperator::InstanceOf => "instanceof",
        });
    }

    /// Print one assignment operator.
    pub(crate) fn write_assign_operator(&mut self, operator: AssignOperator) {
        self.write_punct(match operator {
            AssignOperator::AddAssign => "+=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",
            AssignOperator::ExponentAssign => "**=",
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::ShiftRightAssign => ">>=",
            AssignOperator::UnsignedShiftRightAssign => ">>>=",
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseXorAssign => "^=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        });
    }

    /// Print one string id.
    pub(crate) fn write_string_id(&mut self, string_id: StringId) {
        let string = self.strings.get(string_id);
        self.write_punct(string);
    }

    /// Print one quoted string literal.
    pub(crate) fn write_string_literal(&mut self, string_id: StringId) {
        let encoded = self.encode_js_string_literal(string_id);
        self.write_punct(&encoded);
    }

    /// Print one identifier with one exact source span.
    pub(crate) fn write_identifier_with_source_span(
        &mut self,
        string_id: StringId,
        source_span: Option<Span>,
    ) {
        if let Some(source_span) = source_span {
            self.mark_source(source_span.start);
        }

        self.write_string_id(string_id);

        if let Some(source_span) = source_span {
            self.mark_source(source_span.end);
        }
    }

    /// Print one quoted string literal with one exact source span.
    pub(crate) fn write_string_literal_with_source_span(
        &mut self,
        string_id: StringId,
        source_span: Option<Span>,
    ) {
        let encoded = self.encode_js_string_literal(string_id);

        if let Some(source_span) = source_span {
            self.mark_source(source_span.start);
        }

        self.write_punct(&encoded);

        if let Some(source_span) = source_span {
            self.mark_source(source_span.end);
        }
    }

    /// Encode one string id as one quoted JavaScript string literal.
    pub(crate) fn encode_js_string_literal(&self, string_id: StringId) -> String {
        let string = self.strings.get(string_id);
        let single_quote = Self::encode_js_string_literal_with_quote(string, '\'');
        let double_quote = Self::encode_js_string_literal_with_quote(string, '"');

        if single_quote.len() < double_quote.len() {
            single_quote
        } else {
            double_quote
        }
    }

    /// Encode one string with one explicit quote choice.
    fn encode_js_string_literal_with_quote(string: &str, quote: char) -> String {
        let mut encoded = String::with_capacity(string.len() + 2);
        encoded.push(quote);
        let mut previous = None;
        let mut upcoming = string.chars().peekable();

        while let Some(character) = upcoming.next() {
            match character {
                '\0' => encoded.push_str("\\0"),
                '\u{0007}' => encoded.push_str("\\x07"),
                '\u{0008}' => encoded.push_str("\\b"),
                '\u{0009}' => encoded.push_str("\\t"),
                '\u{000A}' => encoded.push_str("\\n"),
                '\u{000B}' => encoded.push_str("\\v"),
                '\u{000C}' => encoded.push_str("\\f"),
                '\u{000D}' => encoded.push_str("\\r"),
                '\u{001B}' => encoded.push_str("\\x1B"),
                '\\' => encoded.push_str("\\\\"),
                '/' if Self::starts_script_close(previous, upcoming.clone()) => {
                    encoded.push_str("\\/");
                }
                '\'' if quote == '\'' => encoded.push_str("\\'"),
                '"' if quote == '"' => encoded.push_str("\\\""),
                '\u{FEFF}' => encoded.push_str("\\uFEFF"),
                '\u{2028}' => encoded.push_str("\\u2028"),
                '\u{2029}' => encoded.push_str("\\u2029"),
                character if character.is_control() => {
                    encoded.push_str(&format!("\\u{:04X}", character as u32));
                }
                character => encoded.push(character),
            }

            previous = Some(character);
        }

        encoded.push(quote);
        encoded
    }

    /// Encode one number as one short JavaScript numeric literal.
    fn encode_js_number_literal(value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_string();
        }

        if value == f64::INFINITY {
            return "1/0".to_string();
        }

        if value == f64::NEG_INFINITY {
            return "-1/0".to_string();
        }

        let decimal = value.to_string();
        let decimal = Self::minify_decimal_literal(decimal);
        let exponent = Self::minify_decimal_exponent_literal(&decimal);

        if !Self::is_integer_literal(&decimal) {
            if let Some(exponent) = exponent
                && exponent.len() < decimal.len()
            {
                return exponent;
            }

            return decimal;
        }

        let exponent = Self::minify_integer_exponent_literal(&decimal);

        if exponent.len() < decimal.len() {
            exponent
        } else {
            decimal
        }
    }

    /// Return whether one numeric literal string is one plain integer.
    fn is_integer_literal(literal: &str) -> bool {
        !literal.contains(['.', 'e', 'E'])
    }

    /// Remove one unnecessary leading zero before one decimal point.
    fn minify_decimal_literal(literal: String) -> String {
        if let Some(rest) = literal.strip_prefix("0.") {
            return format!(".{rest}");
        }

        if let Some(rest) = literal.strip_prefix("-0.") {
            return format!("-.{rest}");
        }

        literal
    }

    /// Convert one decimal literal into one shorter exponent literal when possible.
    fn minify_decimal_exponent_literal(literal: &str) -> Option<String> {
        let (sign, digits) = if let Some(rest) = literal.strip_prefix('-') {
            ("-", rest)
        } else {
            ("", literal)
        };
        let (integer, fraction) = digits.split_once('.')?;

        let significant_fraction = fraction.trim_end_matches('0');
        if significant_fraction.is_empty() {
            return None;
        }

        let significant_digits = format!("{integer}{significant_fraction}");
        let significant_digits = significant_digits.trim_start_matches('0');
        if significant_digits.is_empty() {
            return None;
        }

        let exponent = -(fraction.len() as i32);

        Some(format!("{sign}{significant_digits}e{exponent}"))
    }

    /// Convert one trailing-zero integer literal into one shorter exponent literal when possible.
    fn minify_integer_exponent_literal(literal: &str) -> String {
        let (sign, digits) = if let Some(rest) = literal.strip_prefix('-') {
            ("-", rest)
        } else {
            ("", literal)
        };

        let trailing_zero_count = digits
            .as_bytes()
            .iter()
            .rev()
            .take_while(|digit| **digit == b'0')
            .count();

        if trailing_zero_count == 0 {
            return literal.to_string();
        }

        let mantissa = &digits[..digits.len() - trailing_zero_count];
        format!("{sign}{mantissa}e{trailing_zero_count}")
    }

    /// Write one punctuation or token fragment.
    pub(crate) fn write_punct(&mut self, text: &str) {
        if self.needs_separator_before(text) {
            self.code.push(' ');
        }

        self.code.push_str(text);
    }

    /// Return whether one separator is needed before one next token fragment.
    pub(crate) fn needs_separator_before(&self, next: &str) -> bool {
        let previous = self.code.chars().next_back();
        let next = next.chars().next();

        Self::needs_separator_between(previous, next)
    }

    /// Return whether one separator is needed between two token boundaries.
    fn needs_separator_between(previous: Option<char>, next: Option<char>) -> bool {
        let (Some(previous), Some(next)) = (previous, next) else {
            return false;
        };

        if Self::is_identifier_char(previous) && Self::is_identifier_char(next) {
            return true;
        }

        if previous.is_ascii_digit() && next == '.' {
            return true;
        }

        matches!(
            (previous, next),
            ('+', '+')
                | ('-', '-')
                | ('/', '/')
                | ('/', '*')
                | ('*', '/')
                | ('<', '<')
                | ('<', '=')
                | ('>', '>')
                | ('>', '=')
                | ('&', '&')
                | ('|', '|')
                | ('?', '?')
                | ('.', '.')
                | ('.', '0'..='9')
                | ('=', '=')
                | ('!', '=')
        )
    }

    /// Return whether one character can appear in one identifier.
    pub(crate) fn is_identifier_char(character: char) -> bool {
        character == '_' || character == '$' || character.is_alphanumeric()
    }

    /// Return whether one slash starts one `</script` sequence.
    fn starts_script_close(previous: Option<char>, upcoming: impl Iterator<Item = char>) -> bool {
        if previous != Some('<') {
            return false;
        }

        let expected = ['s', 'c', 'r', 'i', 'p', 't'];

        upcoming
            .take(expected.len())
            .zip(expected)
            .all(|(actual, expected)| actual.eq_ignore_ascii_case(&expected))
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringPool;
    use destack_fir::format::FileMarker;
    use destack_source::{FileId, FileType, NodeSpanRegion, NodeSpanType, Span};

    use super::Printer;
    use crate::{
        Argument, BinaryOperator, DependencyBinding, DependencyForm, DependencyItem, Expression,
        JsSourceMap, Key, LocalNodeId, LocalNodeIdAny, Name, Path, PostfixPosition, Property,
        ScalarLiteral, Statement, Tree, print_roots_minified, print_roots_minified_with_source_map,
    };

    fn insert_expression(tree: &mut Tree, expression: Expression) -> LocalNodeId<Expression> {
        tree.insert_generated(expression)
    }

    fn insert_property(tree: &mut Tree, property: Property) -> LocalNodeId<Property> {
        tree.insert_generated(property)
    }

    fn insert_argument(tree: &mut Tree, argument: Argument) -> LocalNodeId<Argument> {
        tree.insert_generated(argument)
    }

    fn insert_statement(tree: &mut Tree, statement: Statement) -> LocalNodeId<Statement> {
        tree.insert_generated(statement)
    }

    fn insert_dependency_item(
        tree: &mut Tree,
        item: DependencyItem,
    ) -> LocalNodeId<DependencyItem> {
        tree.insert_generated(item)
    }

    fn build_path(strings: &StringPool, segments: &[&str]) -> Path {
        let segments = segments
            .iter()
            .map(|segment| strings.intern(segment))
            .collect();

        Path { segments }
    }

    fn print_javascript_roots_minified(
        tree: &Tree,
        roots: &[LocalNodeIdAny],
        strings: &StringPool,
    ) -> String {
        let printed = print_roots_minified(FileType::JavaScript, tree, roots, strings).unwrap();

        printed.code
    }

    #[derive(Debug)]
    struct FixedSourceMap {
        node_id: u32,
        span: Span,
    }

    impl JsSourceMap for FixedSourceMap {
        fn source_span(&self, tree: &Tree, node_id: u32) -> Option<Span> {
            let _ = tree;
            let _ = node_id;

            None
        }

        fn source_part_span(
            &self,
            tree: &Tree,
            node_id: u32,
            span_type: NodeSpanType,
        ) -> Option<Span> {
            let _ = tree;

            if node_id == self.node_id && span_type == NodeSpanType::Main {
                return Some(self.span);
            }

            None
        }
    }

    #[derive(Debug)]
    struct FixedPartSourceMap {
        parts: Vec<(u32, NodeSpanType, Span)>,
    }

    impl JsSourceMap for FixedPartSourceMap {
        fn source_span(&self, tree: &Tree, node_id: u32) -> Option<Span> {
            let _ = tree;
            let _ = node_id;

            None
        }

        fn source_part_span(
            &self,
            tree: &Tree,
            node_id: u32,
            span_type: NodeSpanType,
        ) -> Option<Span> {
            let _ = tree;

            self.parts
                .iter()
                .find(|(part_node_id, part_span_type, _)| {
                    *part_node_id == node_id && *part_span_type == span_type
                })
                .map(|(_, _, span)| *span)
        }
    }

    /// Preserve one short numeric spelling for compact output.
    #[test]
    fn test_minifies_number_literal_spellings() {
        assert_eq!(Printer::encode_js_number_literal(1000.0), "1e3");
        assert_eq!(Printer::encode_js_number_literal(0.01), ".01");
        assert_eq!(Printer::encode_js_number_literal(0.0001), "1e-4");
        assert_eq!(Printer::encode_js_number_literal(0.0012), ".0012");
        assert_eq!(Printer::encode_js_number_literal(f64::INFINITY), "1/0");
        assert_eq!(Printer::encode_js_number_literal(f64::NEG_INFINITY), "-1/0");
        assert_eq!(Printer::encode_js_number_literal(f64::NAN), "NaN");
    }

    /// Preserve one short safe string spelling for compact output.
    #[test]
    fn test_minifies_string_literal_spellings() {
        assert_eq!(
            Printer::encode_js_string_literal_with_quote("alpha\"beta", '\''),
            "'alpha\"beta'"
        );
        assert_eq!(
            Printer::encode_js_string_literal_with_quote("</script>", '"'),
            "\"<\\/script>\""
        );
        assert_eq!(
            Printer::encode_js_string_literal_with_quote("\u{2028}\u{2029}\u{FEFF}", '"'),
            "\"\\u2028\\u2029\\uFEFF\""
        );
    }

    /// Preserve one valid separator only where adjacent tokens would merge.
    #[test]
    fn test_inserts_separators_only_for_token_hazards() {
        assert!(Printer::needs_separator_between(Some('a'), Some('b')));
        assert!(Printer::needs_separator_between(Some('+'), Some('+')));
        assert!(Printer::needs_separator_between(Some('1'), Some('.')));
        assert!(Printer::needs_separator_between(Some('/'), Some('*')));

        assert!(!Printer::needs_separator_between(Some('a'), Some('+')));
        assert!(!Printer::needs_separator_between(Some(')'), Some('{')));
        assert!(!Printer::needs_separator_between(Some(']'), Some('.')));
        assert!(!Printer::needs_separator_between(Some('?'), Some('.')));
    }

    /// Print meta-property roots and members through the direct minified printer.
    #[test]
    fn test_prints_meta_property_expressions_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let import_meta = insert_expression(&mut tree, Expression::ImportMeta);
        let import_meta_url = insert_expression(
            &mut tree,
            Expression::Member {
                left: import_meta,
                name: strings.intern("url"),
            },
        );
        let new_target = insert_expression(&mut tree, Expression::NewTarget);
        let new_target_name = insert_expression(
            &mut tree,
            Expression::Member {
                left: new_target,
                name: strings.intern("name"),
            },
        );
        let printed = print_javascript_roots_minified(
            &tree,
            &[import_meta_url.into_any(), new_target_name.into_any()],
            &strings,
        );

        assert_eq!(printed, "import.meta.url;new.target.name");
    }

    /// Print dynamic import attributes through the direct minified printer.
    #[test]
    fn test_prints_dynamic_import_call_with_attributes_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let target = insert_expression(
            &mut tree,
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(strings.intern("./data.json")),
            },
        );
        let type_value = insert_expression(
            &mut tree,
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(strings.intern("json")),
            },
        );
        let type_property = insert_property(
            &mut tree,
            Property::Field {
                modifiers: None,
                key: Key::Name(Name::Identifier(strings.intern("type"))),
                value: type_value,
                is_shorthand: false,
            },
        );
        let with_value = insert_expression(
            &mut tree,
            Expression::ObjectLiteral {
                properties: vec![type_property],
            },
        );
        let with_property = insert_property(
            &mut tree,
            Property::Field {
                modifiers: None,
                key: Key::Name(Name::Identifier(strings.intern("with"))),
                value: with_value,
                is_shorthand: false,
            },
        );
        let options = insert_expression(
            &mut tree,
            Expression::ObjectLiteral {
                properties: vec![with_property],
            },
        );
        let options_argument = insert_argument(&mut tree, Argument::Positional { value: options });
        let import_call = insert_expression(
            &mut tree,
            Expression::ImportCall {
                target,
                target_module: None,
                arguments: vec![options_argument],
            },
        );
        let printed = print_javascript_roots_minified(&tree, &[import_call.into_any()], &strings);

        assert_eq!(printed, "import(\"./data.json\",{with:{type:\"json\"}})");
    }

    /// Keep path imports and runtime imports distinct while minifying.
    #[test]
    fn test_prints_path_and_dynamic_import_roots_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let path = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["import", "meta"]),
                generic_arguments: vec![],
            },
        );
        let target = insert_expression(
            &mut tree,
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(strings.intern("./feature.js")),
            },
        );
        let import_call = insert_expression(
            &mut tree,
            Expression::ImportCall {
                target,
                target_module: None,
                arguments: Vec::new(),
            },
        );
        let printed = print_javascript_roots_minified(
            &tree,
            &[path.into_any(), import_call.into_any()],
            &strings,
        );

        assert_eq!(printed, "import.meta;import(\"./feature.js\")");
    }

    /// Print optional chaining and nullish coalescing without introducing separator hazards.
    #[test]
    fn test_prints_optional_chaining_and_nullish_coalescing_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let object = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["foo"]),
                generic_arguments: vec![],
            },
        );
        let optional_object = insert_expression(
            &mut tree,
            Expression::Maybe {
                position: PostfixPosition::Direct,
                left: object,
            },
        );
        let member = insert_expression(
            &mut tree,
            Expression::Member {
                left: optional_object,
                name: strings.intern("bar"),
            },
        );
        let fallback = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["fallback"]),
                generic_arguments: vec![],
            },
        );
        let expression = insert_expression(
            &mut tree,
            Expression::Binary {
                left: member,
                operator: BinaryOperator::Coalesce,
                right: fallback,
            },
        );
        let printed = print_javascript_roots_minified(&tree, &[expression.into_any()], &strings);

        assert_eq!(printed, "foo?.bar??fallback");
    }

    /// Preserve parentheses for nullish coalescing against logical operators.
    #[test]
    fn test_prints_nullish_coalescing_precedence_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let a = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["a"]),
                generic_arguments: vec![],
            },
        );
        let b = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["b"]),
                generic_arguments: vec![],
            },
        );
        let c = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["c"]),
                generic_arguments: vec![],
            },
        );
        let b_or_c = insert_expression(
            &mut tree,
            Expression::Binary {
                left: b,
                operator: BinaryOperator::Coalesce,
                right: c,
            },
        );
        let a_and_group = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a,
                operator: BinaryOperator::And,
                right: b_or_c,
            },
        );
        let a_and_b = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a,
                operator: BinaryOperator::And,
                right: b,
            },
        );
        let grouped_and_or_c = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a_and_b,
                operator: BinaryOperator::Coalesce,
                right: c,
            },
        );
        let a_or_group = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a,
                operator: BinaryOperator::Or,
                right: b_or_c,
            },
        );
        let a_or_b = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a,
                operator: BinaryOperator::Or,
                right: b,
            },
        );
        let grouped_or_or_c = insert_expression(
            &mut tree,
            Expression::Binary {
                left: a_or_b,
                operator: BinaryOperator::Coalesce,
                right: c,
            },
        );
        let printed = print_javascript_roots_minified(
            &tree,
            &[
                a_and_group.into_any(),
                grouped_and_or_c.into_any(),
                a_or_group.into_any(),
                grouped_or_or_c.into_any(),
            ],
            &strings,
        );

        assert_eq!(printed, "a&&(b??c);(a&&b)??c;a||(b??c);(a||b)??c");
    }

    /// Preserve grouping around optional chaining before one following member access.
    #[test]
    fn test_prints_grouped_optional_chaining_members_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let foo = insert_expression(
            &mut tree,
            Expression::Path {
                path: build_path(&strings, &["foo"]),
                generic_arguments: vec![],
            },
        );
        let optional_foo = insert_expression(
            &mut tree,
            Expression::Maybe {
                position: PostfixPosition::Direct,
                left: foo,
            },
        );
        let optional_member = insert_expression(
            &mut tree,
            Expression::Member {
                left: optional_foo,
                name: strings.intern("bar"),
            },
        );
        let plain_chain = insert_expression(
            &mut tree,
            Expression::Member {
                left: optional_member,
                name: strings.intern("baz"),
            },
        );
        let grouped_optional_member = insert_expression(
            &mut tree,
            Expression::Parenthesized {
                expression: optional_member,
            },
        );
        let grouped_chain = insert_expression(
            &mut tree,
            Expression::Member {
                left: grouped_optional_member,
                name: strings.intern("baz"),
            },
        );
        let printed = print_javascript_roots_minified(
            &tree,
            &[plain_chain.into_any(), grouped_chain.into_any()],
            &strings,
        );

        assert_eq!(printed, "foo?.bar.baz;(foo?.bar).baz");
    }

    /// Mark dynamic import targets with exact source map spans.
    #[test]
    fn test_marks_dynamic_import_target_source_ranges() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let target = insert_expression(
            &mut tree,
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(strings.intern("./feature.js")),
            },
        );
        let import_call = insert_expression(
            &mut tree,
            Expression::ImportCall {
                target,
                target_module: None,
                arguments: Vec::new(),
            },
        );
        let source_map = FixedSourceMap {
            node_id: import_call.id,
            span: Span::new(FileId::new(1), 10, 23),
        };
        let printed = print_roots_minified_with_source_map(
            FileType::JavaScript,
            &tree,
            &[import_call.into_any()],
            &strings,
            &source_map,
        )
        .unwrap();

        assert_eq!(printed.code, "import(\"./feature.js\")");
        assert_eq!(
            printed.markers,
            vec![
                FileMarker {
                    source: 10,
                    dest: 7,
                },
                FileMarker {
                    source: 23,
                    dest: 21,
                },
            ]
        );
    }

    /// Mark import targets and dependency item parts with exact source ranges.
    #[test]
    fn test_marks_import_dependency_part_source_ranges() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        let item = insert_dependency_item(
            &mut tree,
            DependencyItem {
                binding: DependencyBinding::Named,
                form: Some(DependencyForm::Type),
                name: Some(Name::Identifier(strings.intern("value"))),
                alias: Some(strings.intern("alias")),
                value: None,
            },
        );
        let statement = insert_statement(
            &mut tree,
            Statement::Import {
                form: DependencyForm::Plain,
                target: strings.intern("./shared.js"),
                target_module: None,
                items: Some(vec![item]),
                attributes: None,
            },
        );
        let source_map = FixedPartSourceMap {
            parts: vec![
                (
                    statement.id,
                    NodeSpanType::Main,
                    Span::new(FileId::new(1), 20, 33),
                ),
                (
                    item.id,
                    NodeSpanType::Region(NodeSpanRegion::Type),
                    Span::new(FileId::new(1), 7, 12),
                ),
                (
                    item.id,
                    NodeSpanType::Main,
                    Span::new(FileId::new(1), 16, 21),
                ),
            ],
        };
        let printed = print_roots_minified_with_source_map(
            FileType::TypeScript,
            &tree,
            &[statement.into_any()],
            &strings,
            &source_map,
        )
        .unwrap();

        assert_eq!(
            printed.code,
            "import{type value as alias}from\"./shared.js\";"
        );
        assert_eq!(
            printed.markers,
            vec![
                FileMarker {
                    source: 7,
                    dest: 11
                },
                FileMarker {
                    source: 12,
                    dest: 17
                },
                FileMarker {
                    source: 16,
                    dest: 20
                },
                FileMarker {
                    source: 21,
                    dest: 26
                },
                FileMarker {
                    source: 20,
                    dest: 31
                },
                FileMarker {
                    source: 33,
                    dest: 44
                },
            ]
        );
    }
}
