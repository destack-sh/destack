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
        $crate::format::Arguments::new(&[
            $(
                $crate::format::Argument::new(&$value)
            ),+
        ])
    }
}

/// Writes formatted data into a buffer.
///
/// This macro accepts a 'buffer' and a list of format arguments. Each argument will be formatted
/// and the result will be passed to the buffer. The writer may be any value with a `write_format` method;
/// generally this comes from an implementation of the [`crate::Buffer`] trait.
#[macro_export]
macro_rules! write {
    ($dst:expr, [$($arg:expr),+ $(,)?]) => {{
        $dst.write_format($crate::format_args!($($arg),+))
    }}
}

/// Creates the Format IR for a value.
///
/// The first argument `format!` receives is the [`crate::FormatContext`] that specify how nodes must be formatted.
/// Additional parameters passed get formatted by using their [`crate::Format`] implementation.
#[macro_export]
macro_rules! format {
    ($context:expr, [$($arg:expr),+ $(,)?]) => {{
        ($crate::format::format($context, $crate::format_args!($($arg),+)))
    }}
}

/// Provides multiple different alternatives and the printer picks the first one that fits.
/// Use this as last resort because it requires that the printer must try all variants in the worst case.
/// The passed variants must be in the following order:
/// - First: The variant that takes up most space horizontally
/// - Last: The variant that takes up the least space horizontally by splitting the content over multiple lines.
///
/// ## Complexity
/// Be mindful of using this IR node as it has a considerable performance penalty:
/// - There are multiple representation for the same content. This results in increased memory usage
///   and traversal time in the printer.
/// - The worst case complexity is that the printer tires each variant. This can result in quadratic
///   complexity if used in nested structures.
///
/// ## Behavior
/// This IR is similar to the `conditionalGroup` node used by other formatter IRs. The printer measures each variant, except the [`MostExpanded`], in [`Flat`] mode
/// to find the first variant that fits and prints this variant in [`Flat`] mode. If no variant fits, then
/// the printer falls back to printing the [`MostExpanded`] variant in [`Expanded`] mode.
///
/// The declaration of *fits* differs to groups in that the printer only tests if it is possible to print
/// the content up to the first non-soft line break without exceeding the configured print width.
/// This declaration differs from groups as that non-soft line breaks make group expand.
///
/// [`crate::BestFitting`] acts as a "break" boundary, meaning that it is considered to fit
///
/// [`Flat`]: crate::format_node::PrintMode::Flat
/// [`Expanded`]: crate::format_node::PrintMode::Expanded
/// [`MostExpanded`]: crate::format_node::BestFittingVariants::most_expanded
#[macro_export]
macro_rules! best_fitting {
    ($least_expanded:expr, $($tail:expr),+ $(,)?) => {
        $crate::format::BestFitting::from_arguments_unchecked($crate::format_args!($least_expanded, $($tail),+))
    }
}

#[cfg(test)]
mod tests {
    use destack_source::FileType;

    use crate::format::{
        BestFittingMode, FormatState, Formatted, IndentStyle, SimpleFormatOptions, VecBuffer,
    };
    use crate::prelude::*;

    struct TestFormat;

    impl Format<SimpleFormatContext> for TestFormat {
        fn format(&self, f: &mut Formatter<'_, SimpleFormatContext>) -> FormatResult<()> {
            write!(f, [token("test")])
        }
    }

    /// Write a single format node to buffer.
    #[test]
    fn test_single_node() {
        let mut state = FormatState::new(SimpleFormatContext::empty_destack());
        let mut buffer = VecBuffer::new(&mut state);

        write![&mut buffer, [TestFormat]].unwrap();

        assert_eq!(buffer.into_vec(), vec![FormatNode::Token { text: "test" }]);
    }

    /// Write multiple format nodes to buffer.
    #[test]
    fn test_multiple_nodes() {
        let mut state = FormatState::new(SimpleFormatContext::empty_destack());
        let mut buffer = VecBuffer::new(&mut state);

        write![
            &mut buffer,
            [token("a"), space(), token("simple"), space(), TestFormat]
        ]
        .unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![
                FormatNode::Token { text: "a" },
                FormatNode::Space,
                FormatNode::Token { text: "simple" },
                FormatNode::Space,
                FormatNode::Token { text: "test" }
            ]
        );
    }

    /// Format arguments can be used in Format contexts.
    #[test]
    fn test_format_args_basic() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [format_args!(token("Hello World"))]
        )
        .unwrap();

        assert_eq!("Hello World", formatted.print().unwrap().as_str());
    }

    /// Write macro accepts buffer and format arguments.
    #[test]
    fn test_write_macro_basic() {
        let mut state = FormatState::new(SimpleFormatContext::empty_destack());
        let mut buffer = VecBuffer::new(&mut state);

        write!(&mut buffer, [token("Hello"), space()]).unwrap();
        write!(&mut buffer, [token("World")]).unwrap();

        assert_eq!(
            buffer.into_vec(),
            vec![
                FormatNode::Token { text: "Hello" },
                FormatNode::Space,
                FormatNode::Token { text: "World" },
            ]
        );
    }

    /// Format macro creates formatted document from arguments.
    #[test]
    fn test_format_macro_basic() {
        let formatted = format!(SimpleFormatContext::empty_destack(), [token("test")]).unwrap();

        assert_eq!("test", formatted.print().unwrap().as_str());
    }

    /// Format macro respects context options like line width.
    #[test]
    fn test_format_macro_with_options() {
        let options = SimpleFormatOptions {
            indent_style: IndentStyle::Tab,
            line_width: 10,
            ..SimpleFormatOptions::default()
        };
        let context = SimpleFormatContext::new(options, File::empty_text(FileType::Destack));

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
    fn test_best_fitting_with_two_variants_in_first_line_mode() {
        let document = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
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
                File::empty_text(FileType::Destack),
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
                File::empty_text(FileType::Destack),
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
    fn test_best_fitting_with_three_variants_in_first_line_mode() {
        let formatted = format!(
            SimpleFormatContext::empty_destack(),
            [
                token("aVeryLongIdentifier"),
                best_fitting!(
                    // Everything fits on a single line
                    format_args!(
                        token("("),
                        group(&format_args![
                            token("["),
                            soft_block_indent(&format_args![
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3"),
                            ]),
                            token("]")
                        ]),
                        token(")")
                    ),
                    // Breaks after `[`, but prints all nodes on a single line
                    format_args!(
                        token("("),
                        token("["),
                        block_indent(&token("1, 2, 3")),
                        token("]"),
                        token(")"),
                    ),
                    // Breaks after `[` and prints each node on a single line
                    format_args!(
                        token("("),
                        block_indent(&format_args![
                            token("["),
                            block_indent(&format_args![
                                token("1,"),
                                hard_line_break(),
                                token("2,"),
                                hard_line_break(),
                                token("3"),
                            ]),
                            token("]"),
                        ]),
                        token(")")
                    )
                )
            ]
        )
        .unwrap();

        let document = formatted.into_document();

        // takes the first variant if everything fits on a single line
        assert_eq!(
            "aVeryLongIdentifier([1, 2, 3])",
            Formatted::new(
                document.clone(),
                SimpleFormatContext::new(
                    SimpleFormatOptions {
                        indent_style: IndentStyle::Tab,
                        line_width: 80,
                        ..SimpleFormatOptions::default()
                    },
                    File::empty_text(FileType::Destack)
                )
            )
            .print()
            .unwrap()
            .as_str()
        );

        // takes the second if the first variant doesn't fit on a single line
        // the second variant has some additional line breaks to make sure inner groups don't break
        assert_eq!(
            "aVeryLongIdentifier([\n\t1, 2, 3\n])",
            Formatted::new(
                document.clone(),
                SimpleFormatContext::new(
                    SimpleFormatOptions {
                        indent_style: IndentStyle::Tab,
                        line_width: 21,
                        ..SimpleFormatOptions::default()
                    },
                    File::empty_text(FileType::Destack)
                )
            )
            .print()
            .unwrap()
            .as_str()
        );

        // prints the last option as last resort
        assert_eq!(
            "aVeryLongIdentifier(\n\t[\n\t\t1,\n\t\t2,\n\t\t3\n\t]\n)",
            Formatted::new(
                document.clone(),
                SimpleFormatContext::new(
                    SimpleFormatOptions {
                        indent_style: IndentStyle::Tab,
                        line_width: 20,
                        ..SimpleFormatOptions::default()
                    },
                    File::empty_text(FileType::Destack)
                )
            )
            .print()
            .unwrap()
            .as_str()
        );
    }

    /// Best fitting with mode all variants tries all options.
    #[test]
    fn test_best_fitting_with_three_variants_in_all_lines_mode() {
        let document = format_with(|f| {
            write!(
                f,
                [best_fitting![
                    // Everything fits on a single line
                    format_args!(
                        group(&format_args![
                            token("["),
                            soft_block_indent(&format_args![
                                token("1,"),
                                soft_line_break_or_space(),
                                token("2,"),
                                soft_line_break_or_space(),
                                token("3"),
                            ]),
                            token("]")
                        ]),
                        space(),
                        token("+"),
                        space(),
                        token("aVeryLongIdentifier")
                    ),
                    // Breaks after `[` and prints each elements on a single line
                    // The group is necessary because the variant by default is printed in flat mode and a
                    // hard line break indicates that the content doesn't fit.
                    format_args!(
                        token("["),
                        group(&block_indent(&format_args![
                            token("1,"),
                            hard_line_break(),
                            token("2,"),
                            hard_line_break(),
                            token("3")
                        ]))
                        .should_expand(true),
                        token("]"),
                        space(),
                        token("+"),
                        space(),
                        token("aVeryLongIdentifier")
                    ),
                    // Adds parentheses and indents the body, breaks after the operator
                    format_args!(
                        token("("),
                        block_indent(&format_args![
                            token("["),
                            block_indent(&format_args![
                                token("1,"),
                                hard_line_break(),
                                token("2,"),
                                hard_line_break(),
                                token("3"),
                            ]),
                            token("]"),
                            hard_line_break(),
                            token("+"),
                            space(),
                            token("aVeryLongIdentifier")
                        ]),
                        token(")")
                    )
                ]
                .with_mode(BestFittingMode::AllLines),]
            )
        });

        // Takes the first variant if everything fits on a single line
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 40,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [document.clone()]
        )
        .unwrap();
        assert_eq!(
            "[1, 2, 3] + aVeryLongIdentifier",
            formatted.print().unwrap().as_str()
        );

        // It takes the second if the first variant doesn't fit on a single line. The second variant
        // has some additional line breaks to make sure inner groups don't break
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 23,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [document.clone()]
        )
        .unwrap();
        assert_eq!(
            "[\n\t1,\n\t2,\n\t3\n] + aVeryLongIdentifier",
            formatted.print().unwrap().as_str()
        );

        // Prints the last option as last resort
        let formatted = format!(
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 22,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack)
            ),
            [document.clone()]
        )
        .unwrap();
        assert_eq!(
            "(\n\t[\n\t\t1,\n\t\t2,\n\t\t3\n\t]\n\t+ aVeryLongIdentifier\n)",
            formatted.print().unwrap().as_str()
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
                File::empty_text(FileType::Destack),
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
        let result = Formatted::new(
            document,
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 80,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack),
            ),
        )
        .print()
        .unwrap();

        assert_eq!("expect(a).toMatch([\n\t1,\n\t2,\n\t3\n])", result.as_str());
    }

    /// Best fitting selects the appropriate variant based on line width.
    #[test]
    fn test_best_fitting_selects_variant_by_width() {
        // the second variant below should be selected when printing at a width of 30
        let formatted_best_fitting = format!(
            SimpleFormatContext::empty_destack(),
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

        let best_fitting_code = Formatted::new(
            formatted_best_fitting.into_document(),
            SimpleFormatContext::new(
                SimpleFormatOptions {
                    indent_style: IndentStyle::Tab,
                    line_width: 30,
                    ..SimpleFormatOptions::default()
                },
                File::empty_text(FileType::Destack),
            ),
        )
        .print()
        .expect("Document to be valid")
        .as_str()
        .to_string();

        // verify the second variant was selected and formatted correctly
        assert!(best_fitting_code.contains("Start"));
        assert!(best_fitting_code.contains("End"));
        assert!(!best_fitting_code.contains("Something that will not fit"));
    }

    /// Best fitting variants print identically to equivalent normal format args.
    #[test]
    fn test_best_fitting_prints_like_normal_format_args() {
        // create a best fitting with multiple variants
        let formatted_best_fitting = format!(
            SimpleFormatContext::empty_destack(),
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

        // this matches the IR above except that the `best_fitting` was replaced with
        // the contents of its second variant
        let formatted_normal_list = format!(
            SimpleFormatContext::empty_destack(),
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
                File::empty_text(FileType::Destack),
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
                File::empty_text(FileType::Destack),
            ),
        )
        .print()
        .expect("Document to be valid")
        .as_str()
        .to_string();

        // the variant that "fits" will print its contents as if it were a normal list
        // outside of a BestFitting node
        assert_eq!(best_fitting_code, normal_list_code);
    }
}
