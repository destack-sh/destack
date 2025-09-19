/// Constructs the parameters for other formatting macros.
///
/// This macro takes a list of objects implementing [`crate::Format`]. It will canonicalize the
/// arguments into a single type.
///
/// This macro produces a value of type [`crate::Arguments`]. This value can be passed to
/// the macros within [crate]. All other formatting macros ([`format!`](crate::format!),
/// [`write!`](crate::write!)) are proxied through this one. This macro avoids heap allocations.
///
/// [`Format`]: crate::Format
/// [`Arguments`]: crate::Arguments
#[macro_export]
macro_rules! format_args {
    ($($value:expr),+ $(,)?) => {
        $crate::Arguments::new(&[
            $(
                $crate::Argument::new(&$value)
            ),+
        ])
    }
}

/// Writes formatted data into a buffer.
///
/// This macro accepts a 'buffer' and a list of format arguments. Each argument will be formatted
/// and the result will be passed to the buffer. The writer may be any value with a `write_fmt` method;
/// generally this comes from an implementation of the [`crate::Buffer`] trait.
#[macro_export]
macro_rules! write {
    ($dst:expr, [$($arg:expr),+ $(,)?]) => {{
        let result = $dst.write_fmt($crate::format_args!($($arg),+));
        result
    }}
}

/// Writes formatted data into the given buffer and prints all written elements for a quick and dirty debugging.
///
/// NOTE: The macro is intended as debugging tool and therefore you should avoid having
/// uses of it in version control for long periods (other than in tests and similar). Format output
/// from production code is better done with `[write!]`
#[macro_export]
macro_rules! dbg_write {
    ($dst:expr, [$($arg:expr),+ $(,)?]) => {{
        use $crate::BufferExtensions;
        let mut count = 0;
        let mut inspect = $dst.inspect(|element: &FormatElement| {
            std::eprintln!(
                "[{}:{}][{}] = {element:#?}",
                std::file!(), std::line!(), count
            );
            count += 1;
        });
        let result = inspect.write_fmt($crate::format_args!($($arg),+));
        result
    }}
}

/// Creates the Format IR for a value.
///
/// The first argument `format!` receives is the [`crate::FormatContext`] that specify how elements must be formatted.
/// Additional parameters passed get formatted by using their [`crate::Format`] implementation.
#[macro_export]
macro_rules! format {
    ($context:expr, [$($arg:expr),+ $(,)?]) => {{
        ($crate::format($context, $crate::format_args!($($arg),+)))
    }}
}

/// Provides multiple different alternatives and the printer picks the first one that fits.
/// Use this as last resort because it requires that the printer must try all variants in the worst case.
/// The passed variants must be in the following order:
/// - First: The variant that takes up most space horizontally
/// - Last: The variant that takes up the least space horizontally by splitting the content over multiple lines.
///
/// ## Complexity
/// Be mindful of using this IR element as it has a considerable performance penalty:
/// - There are multiple representation for the same content. This results in increased memory usage
///   and traversal time in the printer.
/// - The worst case complexity is that the printer tires each variant. This can result in quadratic
///   complexity if used in nested structures.
///
/// ## Behavior
/// This IR is similar to Prettier's `conditionalGroup`. The printer measures each variant, except the [`MostExpanded`], in [`Flat`] mode
/// to find the first variant that fits and prints this variant in [`Flat`] mode. If no variant fits, then
/// the printer falls back to printing the [`MostExpanded`] variant in [`Expanded`] mode.
///
/// The definition of *fits* differs to groups in that the printer only tests if it is possible to print
/// the content up to the first non-soft line break without exceeding the configured print width.
/// This definition differs from groups as that non-soft line breaks make group expand.
///
/// [`crate::BestFitting`] acts as a "break" boundary, meaning that it is considered to fit
///
/// [`Flat`]: crate::format_element::PrintMode::Flat
/// [`Expanded`]: crate::format_element::PrintMode::Expanded
/// [`MostExpanded`]: crate::format_element::BestFittingVariants::most_expanded
#[macro_export]
macro_rules! best_fitting {
    ($least_expanded:expr, $($tail:expr),+ $(,)?) => {
        // OK because the macro syntax requires at least two variants.
        $crate::BestFitting::from_arguments_unchecked($crate::format_args!($least_expanded, $($tail),+))
    }
}

#[cfg(test)]
mod tests {
    use crate::{IndentStyle, prelude::*};
    use crate::{
        FormatState, Formatted, SimpleFormatOptions, VecBuffer, format, format_args, write,
    };

    struct TestFormat;

    impl Format<SimpleFormatContext> for TestFormat {
        fn fmt(&self, f: &mut Formatter<'_, SimpleFormatContext>) -> FormatResult<()> {
            write!(f, [token("test")])
        }
    }

    /// Write a single format element to buffer.
    #[test]
    fn test_single_element() {
        let mut state = FormatState::new(SimpleFormatContext::default());
        let mut buffer = VecBuffer::new(&mut state);

        write![&mut buffer, [TestFormat]].unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![FormatElement::Token { text: "test" }]
        );
    }

    /// Write multiple format elements to buffer.
    #[test]
    fn test_multiple_elements() {
        let mut state = FormatState::new(SimpleFormatContext::default());
        let mut buffer = VecBuffer::new(&mut state);

        write![
            &mut buffer,
            [token("a"), space(), token("simple"), space(), TestFormat]
        ]
        .unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![
                FormatElement::Token { text: "a" },
                FormatElement::Space,
                FormatElement::Token { text: "simple" },
                FormatElement::Space,
                FormatElement::Token { text: "test" }
            ]
        );
    }

    /// Format arguments can be used in Format contexts.
    #[test]
    fn test_format_args_basic() {
        let formatted = format!(
            SimpleFormatContext::default(),
            [format_args!(token("Hello World"))]
        )
        .unwrap();

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Write macro accepts buffer and format arguments.
    #[test]
    fn test_write_macro_basic() {
        let mut state = FormatState::new(SimpleFormatContext::default());
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [token("Hello"), space()]).unwrap();
        write!(&mut buffer, [token("World")]).unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![
                FormatElement::Token { text: "Hello" },
                FormatElement::Space,
                FormatElement::Token { text: "World" },
            ]
        );
    }

    /// Debug write macro prints elements while writing.
    #[test]
    fn test_dbg_write_macro() {
        let mut state = FormatState::new(SimpleFormatContext::default());
        let mut buffer = VecBuffer::new(&mut state);

        // NOTE @Testing: this will print debug output during test execution
        dbg_write!(buffer, [token("Hello")]).unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![FormatElement::Token { text: "Hello" }]
        );
    }

    /// Format macro creates formatted document from arguments.
    #[test]
    fn test_format_macro_basic() {
        let formatted = format!(SimpleFormatContext::default(), [token("test")]).unwrap();

        assert_eq!("test", formatted.print().unwrap().as_str());
    }

    /// Format macro respects context options like line width.
    #[test]
    fn test_format_macro_with_options() {
        let options = SimpleFormatOptions {
            line_width: 10.try_into().unwrap(),
            ..SimpleFormatOptions::default()
        };
        let context = SimpleFormatContext::new(options, Source::default());

        let formatted = format!(
            context,
            [
                token("a"),
                soft_line_break_or_space(),
                token("very"),
                soft_line_break_or_space(),
                token("long"),
                soft_line_break_or_space(),
                token("line")
            ]
        )
        .unwrap();

        let result = formatted.print().unwrap();
        // should break due to line width constraint
        assert!(result.as_str().contains('\n'));
    }

    /// Best fitting selects first variant that fits within line width.
    #[test]
    fn test_best_fitting_basic() {
        let document = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                Source::default()
            ),
            [
                token("aVeryLongIdentifier"),
                best_fitting!(
                    // first variant - fits on one line
                    format_args!(token("(1, 2, 3)")),
                    // second variant - breaks into multiple lines
                    format_args!(
                        token("("),
                        soft_block_indent(&format_args!(
                            token("1,"),
                            soft_line_break_or_space(),
                            token("2,"),
                            soft_line_break_or_space(),
                            token("3")
                        )),
                        token(")")
                    )
                )
            ]
        )
        .unwrap();

        // with wide line width, should use first variant
        let wide_result = Formatted::new(
            document.clone().into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
					indent_style: IndentStyle::Tab,
                    line_width: 50,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .unwrap();

        assert_eq!("aVeryLongIdentifier(1, 2, 3)", wide_result.as_str());

        // with narrow line width, should use second variant
        let narrow_result = Formatted::new(
            document.into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
					indent_style: IndentStyle::Tab,
                    line_width: 20,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .unwrap();

        assert_eq!(
            "aVeryLongIdentifier(\n\t1,\n\t2,\n\t3\n)",
            narrow_result.as_str()
        );
    }

    /// Best fitting handles complex multi-variant scenarios.
    #[test]
    fn test_best_fitting_complex_variants() {
        let document = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
            [
                token("aVeryLongIdentifier"),
                best_fitting!(
                    // first variant - everything on one line
                    format_args!(token("([1, 2, 3])")),
                    // second variant - break array but keep call on line
                    format_args!(
                        token("("),
                        group(&format_args!(
                            token("["),
                            soft_block_indent(&format_args!(
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3")
                            )),
                            token("]")
                        )),
                        token(")")
                    ),
                    // third variant - break everything
                    format_args!(
                        token("("),
                        soft_block_indent(&format_args!(
                            token("["),
                            soft_block_indent(&format_args!(
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3")
                            )),
                            token("]")
                        )),
                        token(")")
                    )
                )
            ]
        )
        .unwrap();

        // test different line widths select different variants
        let very_wide = Formatted::new(
            document.clone().into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 50,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .unwrap();

        assert_eq!("aVeryLongIdentifier([1, 2, 3])", very_wide.as_str());

        let medium_width = Formatted::new(
            document.clone().into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 21,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .unwrap();

        assert_eq!(
            "aVeryLongIdentifier([\n\t1, 2, 3\n])",
            medium_width.as_str()
        );

        let narrow_width = Formatted::new(
            document.into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 20,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .unwrap();

        assert_eq!(
            "aVeryLongIdentifier(\n\t[\n\t\t1,\n\t\t2,\n\t\t3\n\t]\n)",
            narrow_width.as_str()
        );
    }

    /// Best fitting works with groups that have should_expand set to true.
    #[test]
    fn test_best_fitting_with_should_expand() {
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
            [best_fitting!(
                // first variant - method call on line but break array
                format_args!(
                    token("expect(a).toMatch("),
                    group(&format_args!(
                        token("["),
                        soft_block_indent(&format_args!(
                            token("1,"),
                            soft_line_break_or_space(),
                            token("2,"),
                            soft_line_break_or_space(),
                            token("3"),
                        )),
                        token("]")
                    ))
                    .should_expand(true),
                    token(")")
                ),
                // second variant - break after opening paren
                format_args!(
                    token("expect(a).toMatch("),
                    group(&soft_block_indent(
                        &group(&format_args!(
                            token("["),
                            soft_block_indent(&format_args!(
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3"),
                            )),
                            token("]")
                        ))
                        .should_expand(true),
                    ))
                    .should_expand(true),
                    token(")")
                ),
            )]
        )
        .unwrap();

        let document = formatted.into_document();
        let result = Formatted::new(document, SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
            .print()
            .unwrap();

        assert_eq!("expect(a).toMatch([\n\t1,\n\t2,\n\t3\n])", result.as_str());
    }

    /// Best fitting variants print identically to normal lists when selected.
    #[test]
    fn best_fitting_variants_print_as_lists() {
        // The second variant below should be selected when printing at a width of 30
        let formatted_best_fitting = format!(
            SimpleFormatContext::default(),
            [
                token("aVeryLongIdentifier"),
                soft_line_break_or_space(),
                best_fitting![
                    format_args![token(
                        "Something that will not fit on a line with 30 character print width."
                    )],
                    format_args![
                        group(&format_args![
                            token("Start"),
                            soft_line_break(),
                            group(&soft_block_indent(&format_args![
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3"),
                            ])),
                            soft_line_break_or_space(),
                            soft_block_indent(&format_args![
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                group(&format_args!(
                                    token("A,"),
                                    soft_line_break_or_space(),
                                    token("B")
                                )),
                                soft_line_break_or_space(),
                                token("3")
                            ]),
                            soft_line_break_or_space(),
                            token("End")
                        ])
                        .should_expand(true)
                    ],
                    format_args!(token("Most"), hard_line_break(), token("Expanded"))
                ]
            ]
        )
        .unwrap();

        // This matches the IR above except that the `best_fitting` was replaced with
        // the contents of its second variant.
        let formatted_normal_list = format!(
            SimpleFormatContext::default(),
            [
                token("aVeryLongIdentifier"),
                soft_line_break_or_space(),
                format_args![
                    token("Start"),
                    soft_line_break(),
                    &group(&soft_block_indent(&format_args![
                        token("1,"),
                        soft_line_break_or_space(),
                        token("2,"),
                        soft_line_break_or_space(),
                        token("3"),
                    ])),
                    soft_line_break_or_space(),
                    &soft_block_indent(&format_args![
                        token("1,"),
                        soft_line_break_or_space(),
                        token("2,"),
                        soft_line_break_or_space(),
                        group(&format_args!(
                            token("A,"),
                            soft_line_break_or_space(),
                            token("B")
                        )),
                        soft_line_break_or_space(),
                        token("3")
                    ]),
                    soft_line_break_or_space(),
                    token("End")
                ],
            ]
        )
        .unwrap();

        let best_fitting_code = Formatted::new(
            formatted_best_fitting.into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 30,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .expect("Document to be valid")
        .as_str()
        .to_string();

        let normal_list_code = Formatted::new(
            formatted_normal_list.into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 30,
                    ..SimpleFormatOptions::default()
                },
                Source::default(),
            ),
        )
        .print()
        .expect("Document to be valid")
        .as_str()
        .to_string();

        // The variant that "fits" will print its contents as if it were a normal list
        // outside of a BestFitting element.
        assert_eq!(best_fitting_code, normal_list_code);
    }
}
