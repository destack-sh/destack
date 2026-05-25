use super::*;

/// Tree literals after type aliases should enter tree tokenization at the next tree expression.
#[test]
fn test_lex_tree_after_type_alias_before_tree() {
    let src = "type X = typeof Array\n<div>a</div>";
    let (semantic_tokens, _, _) = lex_source_with_tree_literals(src, LanguageType::TypeScriptXml);
    let tokens: Vec<_> = semantic_tokens
        .iter()
        .map(|token| (token.token.ty(), token.token.literal()))
        .collect();
    let expected = [
        (TokenType::LessThan, None),
        (TokenType::Identifier, None),
        (TokenType::GreaterThan, None),
        (TokenType::Literal, Some(TokenLiteral::TreeString)),
        (TokenType::LessThan, None),
        (TokenType::Divide, None),
        (TokenType::Identifier, None),
        (TokenType::GreaterThan, None),
    ];
    let has_tree_span = tokens
        .windows(expected.len())
        .any(|window| window == expected);
    assert!(
        has_tree_span,
        "expected tree literal tokens after type alias"
    );
}

/// HTML entities outside tree text should remain ordinary tokens.
#[test]
fn test_lex_html_entities_outside_tree_not_decoded() {
    assert_tokenize_eq_roundtrip!(
        "&nbsp;",
        Token::new(TokenType::ElementwiseAnd, 1, None),
        Token::new(TokenType::Identifier, 4, None),
        Token::new(TokenType::Semicolon, 1, None),
    );
}

/// HTML entities inside tree text should decode into tree string tokens.
#[test]
fn test_lex_html_entities_inside_tree_content() {
    assert_tree_tokenize_eq_roundtrip!(
        "<div>&nbsp;</div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Character {
                is_terminated: true,
                is_html_entity: true,
            })
        ), // &nbsp;
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Named, decimal, and hexadecimal HTML entities should decode inside tree text.
#[test]
fn test_lex_html_entities_various_inside_tree() {
    assert_tree_tokenize_eq_roundtrip!(
        "<p>&#160;&#xA0;&amp;</p>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // p
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Character {
                is_terminated: true,
                is_html_entity: true,
            })
        ), // &#160;
        Token::new(
            TokenType::Literal,
            6,
            Some(TokenLiteral::Character {
                is_terminated: true,
                is_html_entity: true,
            })
        ), // &#xA0;
        Token::new(
            TokenType::Literal,
            5,
            Some(TokenLiteral::Character {
                is_terminated: true,
                is_html_entity: true,
            })
        ), // &amp;
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 1, None),  // p
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Entity-like text without semicolons should remain ordinary tokens outside tree text.
#[test]
fn test_lex_entity_like_text_without_semicolon_outside_tree_not_decoded() {
    assert_tokenize_eq_roundtrip!(
        "&nbsp",
        Token::new(TokenType::ElementwiseAnd, 1, None),
        Token::new(TokenType::Identifier, 4, None),
    );
}

/// Self-closing tree tags should lex their closing slash as tree syntax.
#[test]
fn test_lex_tree_self_closing() {
    // <A/> - self-closing tree tag
    assert_tree_tokenize_eq_roundtrip!(
        "<A/>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree tags should keep unicode identifier spans on UTF-8 boundaries.
#[test]
fn test_lex_tree_unicode_tag_identifier() {
    assert_tree_tokenize_eq_roundtrip!(
        "<µtag µ_>µ_</µtag>",
        Token::new(TokenType::LessThan, 1, None),
        Token::new(TokenType::Identifier, 5, None),
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 3, None),
        Token::new(TokenType::GreaterThan, 1, None),
        Token::new(TokenType::Literal, 3, Some(TokenLiteral::TreeString)),
        Token::new(TokenType::LessThan, 1, None),
        Token::new(TokenType::Divide, 1, None),
        Token::new(TokenType::Identifier, 5, None),
        Token::new(TokenType::GreaterThan, 1, None),
    );
}

/// Tree elements with text should emit tree string content tokens.
#[test]
fn test_lex_tree_with_text_content() {
    // <div>Hello</div> - tag with text content
    assert_tree_tokenize_eq_roundtrip!(
        "<div>Hello</div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 5, Some(TokenLiteral::TreeString)), // Hello
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree closing tags should allow trivia between slash and tag name.
#[test]
fn test_lex_tree_closing_tag_with_trivia() {
    assert_tree_tokenize_eq_roundtrip!(
        "<div>< /div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Whitespace, 1, None),  // (space)
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree closing tags should allow comment trivia before the tag name.
#[test]
fn test_lex_tree_closing_tag_with_comment_trivia() {
    assert_tree_tokenize_eq_roundtrip!(
        "<div>< /*comment*/ /div>",
        Token::new(TokenType::LessThan, 1, None),      // <
        Token::new(TokenType::Identifier, 3, None),    // div
        Token::new(TokenType::GreaterThan, 1, None),   // >
        Token::new(TokenType::LessThan, 1, None),      // <
        Token::new(TokenType::Whitespace, 1, None),    // (space)
        Token::new(TokenType::BlockComment, 11, None), // /*comment*/
        Token::new(TokenType::Whitespace, 1, None),    // (space)
        Token::new(TokenType::Divide, 1, None),        // /
        Token::new(TokenType::Identifier, 3, None),    // div
        Token::new(TokenType::GreaterThan, 1, None),   // >
    );
}

/// Tree expression containers should switch between tree and expression tokenization.
#[test]
fn test_lex_tree_with_expression_container() {
    // <div>{x}</div> - tag with expression container
    assert_tree_tokenize_eq_roundtrip!(
        "<div>{x}</div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::OpenBrace, 1, None),   // {
        Token::new(TokenType::Identifier, 1, None),  // x
        Token::new(TokenType::CloseBrace, 1, None),  // }
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree text around expression containers should preserve tree string boundaries.
#[test]
fn test_lex_tree_with_text_and_expression() {
    // <div>Hello {name}!</div> - mixed text and expression
    assert_tree_tokenize_eq_roundtrip!(
        "<div>Hello {name}!</div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 6, Some(TokenLiteral::TreeString)), // "Hello "
        Token::new(TokenType::OpenBrace, 1, None),   // {
        Token::new(TokenType::Identifier, 4, None),  // name
        Token::new(TokenType::CloseBrace, 1, None),  // }
        Token::new(TokenType::Literal, 1, Some(TokenLiteral::TreeString)), // "!"
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree tags with generic arguments should lex generic punctuation inside the tag.
#[test]
fn test_lex_tree_with_generic_arguments() {
    assert_tokenize_eq_roundtrip!(
        "<Component<any>></Component>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 9, None),  // Component
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // any
        Token::new(TokenType::ShiftRight, 2, None),  // >>
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 9, None),  // Component
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Generic arrow functions should not enter tree tokenization.
#[test]
fn test_lex_generic_arrow_not_tree() {
    assert_tokenize_eq_roundtrip!(
        "<T>(x) => x",
        Token::new(TokenType::LessThan, 1, None),        // <
        Token::new(TokenType::Identifier, 1, None),      // T
        Token::new(TokenType::GreaterThan, 1, None),     // >
        Token::new(TokenType::OpenParenthesis, 1, None), // (
        Token::new(TokenType::Identifier, 1, None),      // x
        Token::new(TokenType::CloseParenthesis, 1, None), // )
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::ArrowWide, 2, None), // =>
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None), // x
    );
}

/// Generic arrow functions with comments should not enter tree tokenization.
#[test]
fn test_lex_generic_arrow_not_tree_with_comment() {
    assert_tokenize_eq_roundtrip!(
        "<T>(/*x*/y) => y",
        Token::new(TokenType::LessThan, 1, None),        // <
        Token::new(TokenType::Identifier, 1, None),      // T
        Token::new(TokenType::GreaterThan, 1, None),     // >
        Token::new(TokenType::OpenParenthesis, 1, None), // (
        Token::new(TokenType::BlockComment, 5, None),    // /*x*/
        Token::new(TokenType::Identifier, 1, None),      // y
        Token::new(TokenType::CloseParenthesis, 1, None), // )
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::ArrowWide, 2, None), // =>
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None), // y
    );
}

/// Less-than comparisons should not enter tree tokenization.
#[test]
fn test_lex_comparison_not_tree() {
    // a < b should be comparison, not tree opening
    assert_tokenize_eq_roundtrip!(
        "a < b",
        Token::new(TokenType::Identifier, 1, None), // a
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::LessThan, 1, None), // <
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::Identifier, 1, None), // b
    );
}

/// Tree literals after return should enter tree tokenization.
#[test]
fn test_lex_tree_after_return() {
    // return <A/> - tree after keyword
    assert_tree_tokenize_eq_roundtrip!(
        "return <A/>",
        Token::new(TokenType::Identifier, 6, None), // return
        Token::new(TokenType::Whitespace, 1, None),
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree literals after open parentheses should enter tree tokenization.
#[test]
fn test_lex_tree_after_open_paren() {
    // (<A/>) - tree in parentheses
    assert_tree_tokenize_eq_roundtrip!(
        "(<A/>)",
        Token::new(TokenType::OpenParenthesis, 1, None), // (
        Token::new(TokenType::LessThan, 1, None),        // <
        Token::new(TokenType::Identifier, 1, None),      // A
        Token::new(TokenType::Divide, 1, None),          // /
        Token::new(TokenType::GreaterThan, 1, None),     // >
        Token::new(TokenType::CloseParenthesis, 1, None), // )
    );
}

/// Tree fragments should lex opening and closing fragment tags.
#[test]
fn test_lex_tree_fragment() {
    // <></> - empty fragment
    assert_tree_tokenize_eq_roundtrip!(
        "<></>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Invalid HTML entities in tree text should remain tree string text.
#[test]
fn test_lex_tree_invalid_html_entity_as_text() {
    assert_tree_tokenize_eq_roundtrip!(
        "<A>&#x1g4q9;</A>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 9, Some(TokenLiteral::TreeString)), // &#x1g4q9;
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// HTML5 entities outside the TypeScript JSX set should remain tree string text.
#[test]
fn test_lex_tree_html5_only_entity_as_text() {
    assert_tree_tokenize_eq_roundtrip!(
        "<A>&Acy;</A>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 5, Some(TokenLiteral::TreeString)), // &Acy;
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Nested tree elements should preserve tree tag and text token boundaries.
#[test]
fn test_lex_tree_nested() {
    // <A><B/></A> - nested tree element
    assert_tree_tokenize_eq_roundtrip!(
        "<A><B/></A>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // B
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Nested tree elements should preserve whitespace as tree string text.
#[test]
fn test_lex_tree_nested_with_whitespace() {
    // nested with whitespace
    assert_tree_tokenize_eq_roundtrip!(
        "<A>\n    <B/>\n</A>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 5, Some(TokenLiteral::TreeString)), // "\n    " (whitespace)
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 1, None),  // B
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 1, Some(TokenLiteral::TreeString)), // "\n" (whitespace)
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 1, None),  // A
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Deeply nested tree elements should keep balanced tree tokenization.
#[test]
fn test_lex_tree_deeply_nested() {
    let input = r"
<A>
    <B>
        <C>
            <D/>
            {2}
        </C>
    </B>
</A>
";
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    let opens: Vec<_> = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::LessThan)
        .collect();
    assert_eq!(opens.len(), 7, "should have 7 < tokens");

    let divides: Vec<_> = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::Divide)
        .collect();
    assert_eq!(
        divides.len(),
        4,
        "should have 4 / tokens (1 self-close + 3 closing)"
    );

    // verify expression container tokens exist
    let open_braces: Vec<_> = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::OpenBrace)
        .collect();
    assert_eq!(open_braces.len(), 1, "should have 1 open brace token");

    let close_braces: Vec<_> = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::CloseBrace)
        .collect();
    assert_eq!(close_braces.len(), 1, "should have 1 close brace token");
}

/// Tree text containing colons should remain one tree string token.
#[test]
fn test_lex_tree_text_with_colon() {
    // <h4>Tool: {x}</h4> - text containing colon should be lexed as TreeString
    assert_tree_tokenize_eq_roundtrip!(
        "<h4>Tool: {x}</h4>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 2, None),  // h4
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 6, Some(TokenLiteral::TreeString)), // "Tool: "
        Token::new(TokenType::OpenBrace, 1, None),   // {
        Token::new(TokenType::Identifier, 1, None),  // x
        Token::new(TokenType::CloseBrace, 1, None),  // }
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 2, None),  // h4
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Tree attribute expression containers should allow nested child elements.
#[test]
fn test_lex_tree_nested_with_attr_expression() {
    assert_tree_tokenize_eq_roundtrip!(
        "<div key={index}><h4>Tool: {x}</h4></div>",
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::Whitespace, 1, None),  // (space)
        Token::new(TokenType::Identifier, 3, None),  // key
        Token::new(TokenType::Assign, 1, None),      // =
        Token::new(TokenType::OpenBrace, 1, None),   // {
        Token::new(TokenType::Identifier, 5, None),  // index
        Token::new(TokenType::CloseBrace, 1, None),  // }
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Identifier, 2, None),  // h4
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::Literal, 6, Some(TokenLiteral::TreeString)), // "Tool: "
        Token::new(TokenType::OpenBrace, 1, None),   // {
        Token::new(TokenType::Identifier, 1, None),  // x
        Token::new(TokenType::CloseBrace, 1, None),  // }
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 2, None),  // h4
        Token::new(TokenType::GreaterThan, 1, None), // >
        Token::new(TokenType::LessThan, 1, None),    // <
        Token::new(TokenType::Divide, 1, None),      // /
        Token::new(TokenType::Identifier, 3, None),  // div
        Token::new(TokenType::GreaterThan, 1, None), // >
    );
}

/// Self-closing tree tags should recover after nested attribute object expressions.
#[test]
fn test_lex_tree_self_closing_after_nested_attribute_object_expression() {
    let input = r#"<F
  values={{
    resend: (chunks) => (
      <button>
        {resendCooldown > 0 ? <>{chunks} ({resendCooldown})</> : chunks}
      </button>
    ),
  }}
/>"#;
    let (tokens, _, _) = lex_source_with_tree_literals(input, LanguageType::TypeScriptXml);
    let semantic_types: Vec<TokenType> = tokens
        .iter()
        .map(|token| token.token.ty())
        .filter(|token_type| *token_type != TokenType::Newline)
        .collect();

    assert!(
        semantic_types.ends_with(&[
            TokenType::CloseBrace,
            TokenType::CloseBrace,
            TokenType::Divide,
            TokenType::GreaterThan,
            TokenType::End,
        ]),
        "expected attribute object close followed by self closing tree tag",
    );
}

/// Tree literals inside nested callbacks should keep expression and tree modes balanced.
#[test]
fn test_lex_tree_in_nested_callbacks() {
    let input = r#"x.map((m) => (<>{y.map((p) => { switch (p) { case 'a': return (<div></div>); } })}</>))"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    let open_parens = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::OpenParenthesis)
        .count();
    let close_parens = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::CloseParenthesis)
        .count();

    assert_eq!(open_parens, close_parens, "parentheses should be balanced");
}

/// Nested multiline tree literals should preserve surrounding text tokens.
#[test]
fn test_lex_tree_nested_multiline_with_text() {
    let input = r#"x.map((m) => (
	<>
		{y.map((p, i) => {
			switch (p.type) {
				case 'a':
					return (
						<div key={i}>
							<h4>Tool: {p.name}</h4>
						</div>
					);
			}
		})}
	</>
))"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // verify "Tool: " is lexed as TreeString
    let has_tool_tree_string = tokens.iter().any(|t| {
        t.token.literal() == Some(TokenLiteral::TreeString)
            && input[t.span.start as usize..t.span.end as usize].contains("Tool:")
    });
    assert!(has_tool_tree_string, "expected 'Tool: ' to be a TreeString");

    // verify parentheses are balanced
    let open_parens = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::OpenParenthesis)
        .count();
    let close_parens = tokens
        .iter()
        .filter(|t| t.token.ty() == TokenType::CloseParenthesis)
        .count();
    assert_eq!(open_parens, close_parens, "parentheses should be balanced");
}

/// Tree text after map callback blocks should stay in tree content mode.
#[test]
fn test_lex_tree_text_after_map_callback_blocks() {
    let input = r#"<div>
  <Show when={selectedView() === 'queries'}>
    <select
      value={sort()}
      onChange={(e) => {
        props.setLocalStore('sort', e.currentTarget.value)
      }}
    >
      {Object.keys(sortFns).map((key) => (
        <option value={key}>Sort by {key}</option>
      ))}
    </select>
  </Show>
  <button>
    <Show
      when={
        (selectedView() === 'queries'
          ? sortOrder()
          : mutationSortOrder()) === 1
      }
    >
      <span>Asc</span>
      <ArrowUp />
    </Show>
  </button>
</div>"#;
    let (tokens, _, _) = lex_source_with_tree_literals(input, LanguageType::TypeScriptXml);

    let asc_token = tokens
        .iter()
        .find(|token| &input[token.span.start as usize..token.span.end as usize] == "Asc")
        .expect("expected Asc token");
    assert_eq!(asc_token.token.ty(), TokenType::Literal);
    assert_eq!(asc_token.token.literal(), Some(TokenLiteral::TreeString));
}

/// Tree comment containers should lex their block comments inside expression mode.
#[test]
fn test_lex_tree_with_comment_container() {
    assert_tree_tokenize_eq_roundtrip!(
        "<div>{/* comment */}</div>",
        Token::new(TokenType::LessThan, 1, None),      // <
        Token::new(TokenType::Identifier, 3, None),    // div
        Token::new(TokenType::GreaterThan, 1, None),   // >
        Token::new(TokenType::OpenBrace, 1, None),     // {
        Token::new(TokenType::BlockComment, 13, None), // /* comment */
        Token::new(TokenType::CloseBrace, 1, None),    // }
        Token::new(TokenType::LessThan, 1, None),      // <
        Token::new(TokenType::Divide, 1, None),        // /
        Token::new(TokenType::Identifier, 3, None),    // div
        Token::new(TokenType::GreaterThan, 1, None),   // >
    );
}

/// Tree siblings after expression containers should resume tree content mode.
#[test]
fn test_lex_tree_sibling_after_expr_container() {
    let input = r#"return (
    <div>
        {x.map(() => (<p></p>))}
        <form></form>
    </div>
)"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // The `<form` should be recognized as a tree opening
    let form_start = input.find("<form").unwrap();
    let token_at_form = tokens.iter().find(|t| t.span.start as usize == form_start);

    assert_eq!(
        token_at_form.map(|t| t.token.ty()),
        Some(TokenType::LessThan),
        "< before form should be tree opening"
    );
}

/// Tree siblings after tab-indented expression containers should resume tree content mode.
#[test]
fn test_lex_tree_sibling_after_expr_container_tabs() {
    let input = "return (\n\t<div>\n\t\t{x.map(() => (<p></p>))}\n\t\t<form></form>\n\t</div>\n)";
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // The `<form` should be recognized as a tree opening
    let form_start = input.find("<form").unwrap();
    let token_at_form = tokens.iter().find(|t| t.span.start as usize == form_start);

    assert_eq!(
        token_at_form.map(|t| t.token.ty()),
        Some(TokenType::LessThan),
        "< before form should be tree opening"
    );
}

/// Multiline form trees after expression containers should resume tree content mode.
#[test]
fn test_lex_tree_multiline_form_after_expression_container() {
    let input = "<div>\n\t{x}\n\t<form\n\t\tonClick={() => {}}\n\t>\n\t</form>\n</div>";
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // The `<form` should be recognized as a tree opening
    let form_start = input.find("<form").unwrap();
    let token_at_form = tokens.iter().find(|t| t.span.start as usize == form_start);

    assert_eq!(
        token_at_form.map(|t| t.token.ty()),
        Some(TokenType::LessThan),
        "< in <form should be tree opening"
    );
}

/// Tree tags after nested spread attributes should resume tree tag tokenization.
#[test]
fn test_lex_tree_after_nested_spread_attributes() {
    let input = r#"const labels = [
        ...(a
            ? [
                  <>
                      <span className="flex items-center">
                          <Tooltip
                              title={`text`}
                          >
                              <Icon />
                          </Tooltip>
                      </span>
                  </>,
              ]
            : []),
        ...(b
            ? [
                  <>
                      {y && <E />}
                  </>,
              ]
            : []),
    ]"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // The <E after && should be LessThan (tree opening), not a comparison
    let e_start = input.find("<E").unwrap();
    let token_at_e = tokens.iter().find(|t| t.span.start as usize == e_start);
    assert_eq!(
        token_at_e.map(|t| t.token.ty()),
        Some(TokenType::LessThan),
        "< before E should be tree opening"
    );
}

/// Tree literals after ternary question tokens should enter tree tokenization.
#[test]
fn test_lex_tree_after_ternary_question() {
    let input = r#"a == b ? <>{y && <E />}</> : null"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // Check that <> is recognized as tree fragment opening
    let fragment_start = input.find("<>").unwrap();
    let token_at_fragment = tokens
        .iter()
        .find(|t| t.span.start as usize == fragment_start);
    assert_eq!(
        token_at_fragment.map(|t| t.token.ty()),
        Some(TokenType::LessThan),
        "<> should be recognized as tree opening"
    );
}

/// Tree fragment text after logical and should stay in tree content mode.
#[test]
fn test_lex_tree_fragment_text_after_logical_and() {
    let input = r#"<code>{value && <>x</>}</code>"#;
    let language = LanguageType::TypeScriptXml;
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    let text_token = tokens
        .iter()
        .find(|token| &input[token.span.start as usize..token.span.end as usize] == "x")
        .expect("expected text token inside fragment");

    assert_eq!(text_token.token.ty(), TokenType::Literal);
    assert_eq!(text_token.token.literal(), Some(TokenLiteral::TreeString));
}

/// Tree literals inside attribute expressions should support nested tree tokenization.
#[test]
fn test_lex_tree_nested_in_attribute_expression() {
    let input = r#"<Outer title={<div><LemonButton icon={<IconLink />} /></div>} />"#;
    let language = LanguageType::default();
    let (tokens, _, _) = lex_source_with_tree_literals(input, language);

    // After <IconLink />, the } should be CloseBrace (not TreeString)
    let close_brace_after_icon = tokens
        .iter()
        .find(|t| t.span.start == 50)
        .map(|t| t.token.ty());
    assert_eq!(
        close_brace_after_icon,
        Some(TokenType::CloseBrace),
        "}} after <IconLink /> should be CloseBrace, not TreeString"
    );
}
