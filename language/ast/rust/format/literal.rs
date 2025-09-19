use crate::{DystFormatContext, DystFormatter, ScalarLiteral};
use dyst_language_fir::format::{Format, FormatResult, text, token};

impl Format<DystFormatContext<'_>> for ScalarLiteral {
    fn format(&self, f: &mut DystFormatter<'_, '_>) -> FormatResult<()> {
        match *self {
            ScalarLiteral::Void => token("void").format(f),
            ScalarLiteral::Null => token("null").format(f),
            ScalarLiteral::Boolean(value) => token(if value { "true" } else { "false" }).format(f),
            ScalarLiteral::Byte(value) => text(&value.to_string()).format(f),
            ScalarLiteral::Integer(value, _) => text(&value.to_string()).format(f),
            ScalarLiteral::Float(value, _) => text(&value.to_string()).format(f),
            ScalarLiteral::Character(value) => text(&value.to_string()).format(f),
            _ => todo!("strings!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions};

    #[test]
    fn test_format_integer() {
        let (test, literal_id) = TestFormatter::new("1", |p| p.eat_scalar_literal()).unwrap();
        let literal = &test.tree.get(literal_id);
        let formatted = test.format(literal, DystFormatOptions::default());
        assert_eq!(formatted, "1");
    }
}
