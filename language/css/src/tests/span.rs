use crate::{NodeSpanRegion, NodeSpanType, Rule, Stylesheet, assert_node};

use super::TestParser;

/// Preserve full rule spans and rule side spans from authored source.
#[test]
fn test_store_rule_and_import_side_spans() {
    let test = TestParser::new();
    let source = r#"
        @import "./base.css" layer(theme);
        @media screen {.button { color: red; }}
    "#;
    let (tree, stylesheet) = test.parse_stylesheet(source);

    assert_node!(tree, stylesheet, Stylesheet { rules, .. } => {
        let import_rule = rules[0];
        let media_rule = rules[1];

        assert_node!(tree, import_rule, Rule::Import(_));
        assert_node!(tree, media_rule, Rule::Media(_));

        assert_eq!(
            tree.span(import_rule),
            test.span_for(source, r#"@import "./base.css" layer(theme);"#)
        );
        assert_eq!(
            tree.side_span(
                import_rule,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
            ),
            Some(test.span_for(source, r#"@import "./base.css" layer(theme)"#))
        );
        assert_eq!(
            tree.side_span(
                import_rule,
                NodeSpanType::Region(NodeSpanRegion::Value),
            ),
            Some(test.span_for(source, "./base.css"))
        );
        assert_eq!(
            tree.side_span(
                media_rule,
                NodeSpanType::Region(NodeSpanRegion::Prelude),
            ),
            Some(test.span_for(source, "@media screen"))
        );
    });
}
