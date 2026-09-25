use crate::{ExpressionPosition, ExpressionStop};
use tspp_core::StringId;
use tspp_dir::{
    DependencyBinding, DependencyItem, Expression, ImportAttribute, ImportAttributeClauseKind,
    ImportAttributeValue, Literal, LocalNodeId, Name,
};
use tspp_source::{NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

use crate::parse::ParserErrorKind;
use crate::{Parser, TestParser, assert_expression_path, assert_node, assert_string};

fn assert_import_target_string(parser: &Parser, target: StringId, expected: &str) {
    assert_string!(parser, target, expected);
}

fn import_items(
    items: &Option<Vec<LocalNodeId<DependencyItem>>>,
) -> &[LocalNodeId<DependencyItem>] {
    items.as_deref().expect("expected import specifier shell")
}

fn assert_bare_import(items: &Option<Vec<LocalNodeId<DependencyItem>>>) {
    assert!(items.is_none());
}

fn assert_empty_import_shell(
    items: &Option<Vec<LocalNodeId<DependencyItem>>>,
) -> &[LocalNodeId<DependencyItem>] {
    let items = import_items(items);
    assert!(items.is_empty());
    items
}

#[test]
fn test_parse_import_simple() {
    // import sample
    let test = TestParser::new("import \"tspp\"");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // import
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "tspp");
    });
}

#[test]
fn test_parse_import_from_expression() {
    let test = TestParser::new("import os from 'os'");
    let mut parser = test.prepare();
    let expression_id = parser
        .parse_expression(ExpressionPosition::Value, ExpressionStop::default())
        .unwrap();

    // import os from 'os'
    assert_node!(parser.tree, expression_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "os");
        });
        assert_import_target_string(&parser, *target, "os");
    });
}

#[test]
fn test_parse_import_path_with_arguments() {
    let test = TestParser::new("import \"tspp.geometry\" with { bar: true }");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // import sample.module with { bar: true }
    assert_node!(parser.tree, import_id, Expression::Import { target, items, attributes, .. } => {
        // sample.module
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "tspp.geometry");

        // with { bar: true }
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        let entries = &attributes.attributes;
        assert_eq!(entries.len(), 1);
        assert_string!(parser, entries[0].key.string(), "bar");
        assert_eq!(
            entries[0].value,
            ImportAttributeValue::Literal(Literal::Boolean(true))
        );
    });

    let source = parser.file.text();
    let clause_start = source.find("with").unwrap() as u32;
    let attribute_start = source.find("bar").unwrap() as u32;
    let attribute_end = source.find(" }").unwrap() as u32;

    assert_eq!(
        parser
            .tree
            .get_side_span(import_id, NodeSpanType::Region(NodeSpanRegion::Attributes)),
        Some(Span::new(parser.file.id, clause_start, source.len() as u32)),
    );
    assert_eq!(
        parser
            .tree
            .get_side_span(import_id, NodeSpanType::ListItem(NodeSpanList::Entry, 0)),
        Some(Span::new(parser.file.id, attribute_start, attribute_end,)),
    );
}

#[test]
fn test_parse_import_path_with_missing_attribute_close_brace() {
    let test = TestParser::new("import \"tspp.geometry\" with { bar: true");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        assert_eq!(attributes.attributes.len(), 1);
        assert_string!(parser, attributes.attributes[0].key.string(), "bar");
        assert_eq!(
            attributes.attributes[0].value,
            ImportAttributeValue::Literal(Literal::Boolean(true))
        );
    });
}

#[test]
fn test_parse_import_path_with_nested_attributes() {
    let test = TestParser::new(
        r#"import "tspp.geometry" with {
    mode: "json"
    options: { eager: true, levels: [1, 2] }
}"#,
    );
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        assert_eq!(attributes.attributes.len(), 2);

        assert_string!(parser, attributes.attributes[0].key.string(), "mode");
        assert_eq!(
            attributes.attributes[0].value,
            ImportAttributeValue::Literal(Literal::String(parser.strings.intern("json")))
        );

        assert_string!(parser, attributes.attributes[1].key.string(), "options");
        assert_eq!(
            attributes.attributes[1].value,
            ImportAttributeValue::Object(vec![
                ImportAttribute {
                    key: Name::Identifier(parser.strings.intern("eager")),
                    value: ImportAttributeValue::Literal(Literal::Boolean(true)),
                },
                ImportAttribute {
                    key: Name::Identifier(parser.strings.intern("levels")),
                    value: ImportAttributeValue::Array(vec![
                        ImportAttributeValue::Literal(Literal::Integer(1)),
                        ImportAttributeValue::Literal(Literal::Integer(2)),
                    ]),
                },
            ])
        );
    });
}

#[test]
fn test_parse_import_with_prefix_items() {
    let test = TestParser::new("import { Vector2, Vector3 as V3 } from \"ds.geometry\"");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias, .. } => {
            assert_string!(parser, name.string(), "Vector2");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem::Binding { name: Some(name), alias: Some(alias),.. } => {
            assert_string!(parser, name.string(), "Vector3");
            assert_string!(parser, *alias, "V3");
        });
        assert_import_target_string(&parser, *target, "ds.geometry");
    });
}

#[test]
fn test_parse_import_as_alias() {
    let test = TestParser::new(r#"import * as geom from "ds/geometry""#);
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // import * as geom from ds.geometry
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "geom");
        });
        assert_import_target_string(&parser, *target, "ds/geometry");
    });
}

#[test]
fn test_parse_import_with_newline_before_from() {
    let test = TestParser::new("import { A }\nfrom 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // parse multiline named import with from on the next line
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "A");
            assert!(alias.is_none());
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_before_from() {
    let test = TestParser::new(
        "import HeaderNavigationButton
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // parse multiline default import with from on the next line
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "HeaderNavigationButton");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_block_items() {
    let test = TestParser::new(
        "import Default,
{ Item }
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // parse multiline default plus named imports
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);

        // default binding
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "Default");
        });

        // named binding
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "Item");
            assert!(alias.is_none());
        });

        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_comment_before_from() {
    let test = TestParser::new(
        "import BreakoutRooms
// @ts-ignore
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // parse default import with comment between binding and from
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "BreakoutRooms");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_with_newline_after_from() {
    let test = TestParser::new(
        "import { goBack } from
'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    // parse import target on the next line after from
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "goBack");
            assert!(alias.is_none());
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_multiline_items() {
    let test = TestParser::new(
        "
import {
  StructuredObject,
  StructuredObjectOptions,
} from './lib/object.ng';
",
    );
    let mut parser = test.prepare();

    let import_id = parser.parse_import().unwrap();
    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "./lib/object.ng");

        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias,.. } => {
            assert_string!(parser, name.string(), "StructuredObject");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem::Binding { name: Some(name), alias,.. } => {
            assert_string!(parser, name.string(), "StructuredObjectOptions");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_items_without_separator_spaces() {
    let test = TestParser::new("import {foo,bar,baz} from 'module'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 3);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), .. } => {
            assert_string!(parser, name.string(), "foo");
        });
        assert_node!(parser.tree, items[1], DependencyItem::Binding { name: Some(name), .. } => {
            assert_string!(parser, name.string(), "bar");
        });
        assert_node!(parser.tree, items[2], DependencyItem::Binding { name: Some(name), .. } => {
            assert_string!(parser, name.string(), "baz");
        });
    });
}

#[test]
fn test_parse_import_with_default_and_block() {
    let test = TestParser::new("import Default, { Item } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        // Default
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "Default");
        });
        // { Item }
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias: None,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "Item");
        });
        // `foo`
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_type_identifier_name() {
    // treat type as a value name in named imports
    let test = TestParser::new("import { type } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias, .. } => {
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_type_as_default_name() {
    let test = TestParser::new("import type from './a'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "./a");
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "type");
        });
    });
}

#[test]
fn test_parse_import_named_alias_after_newline_comment() {
    let test = TestParser::new("import {\n  a\n  // keep alias on next line\n  as b\n} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias: Some(alias), .. } => {
            assert_string!(parser, name.string(), "a");
            assert_string!(parser, *alias, "b");
        });
    });
}

#[test]
fn test_parse_bare_import_with_newline_after_comment() {
    let test = TestParser::new("import // keep target on next line\n'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_block_with_newline_after_comment() {
    let test = TestParser::new("import // keep binding on next line\n{} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let _items = assert_empty_import_shell(items);
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_type_as_value_alias() {
    // treat type as a value name with an alias
    let test = TestParser::new("import { type as alias } from './a'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias: Some(alias), .. } => {
            assert_string!(parser, name.string(), "type");
            assert_string!(parser, *alias, "alias");
        });
    });
}

#[test]
fn test_parse_import_block_recovers_broken_alias_item() {
    // preserve valid siblings after one broken import item
    let test = TestParser::new("import { Foo as, Bar } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Error);
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_block_recovers_missing_close_before_from() {
    // preserve the import clause when `}` is omitted before from
    let test = TestParser::new("import { Foo from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_block_keeps_statement_owner_in_missing_close_gap() {
    // keep the import expression enclosing the whitespace gap before `from`
    let source = "import { Widget,  from \"./types.tspp\";";
    let cursor = source.find("  from").unwrap() as u32 + 1;
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();
    let root_id = roots[0];

    assert_node!(parser.tree, root_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, *target, "./types.tspp");
            assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), .. } => {
                assert_string!(parser, name.string(), "Widget");
            });
    });

    let enclosing = parser
        .tree
        .source_index
        .get_enclosing_spans(parser.file_id, cursor, cursor);
    assert!(enclosing.iter().any(|span| span.source_id == root_id.id));
}

#[test]
fn test_parse_import_keeps_target_main_span_for_unterminated_path() {
    // keep the import expression and target main span around the trailing path byte
    let source = "import { } from \"./u";
    let cursor = source.len() as u32;
    let probe = cursor.saturating_sub(1);
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let roots = parser.parse_in_place();
    let root_id = roots[0];

    assert_node!(parser.tree, root_id, Expression::Import { target, .. } => {
            assert_import_target_string(&parser, *target, "./u");
    });

    let enclosing = parser
        .tree
        .source_index
        .get_enclosing_spans(parser.file_id, probe, probe);
    assert!(enclosing.iter().any(|span| span.source_id == root_id.id));

    let main_span = parser.tree.source_index.get_main(root_id.id).unwrap();
    assert!(main_span.contains(probe));
}

#[test]
fn test_parse_export_block_recovers_broken_alias_item() {
    // preserve valid siblings after one broken export item
    let test = TestParser::new("export { Foo as, Bar } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { items, target: Some(target), .. } => {
        assert_eq!(items.len(), 2);
        assert_string!(parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Error);
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_type_identifier_name() {
    // treat type as a value name in named exports
    let test = TestParser::new("export { type } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), alias, .. } => {
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_clause_after_comment_newline_keyword() {
    let test = TestParser::new("export //comment\n{}");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert!(items.is_empty());
    });
}

#[test]
fn test_parse_export_specifier_alias_after_comment_newline() {
    let test = TestParser::new("export {\n  bar as // comment\n  baz,\n} from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert_string!(parser, *alias, "baz");
        });
    });
}

#[test]
fn test_parse_import_specifier_alias_after_comment_newline() {
    let test = TestParser::new("import {\n  bar as // comment\n  baz,\n} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "foo");
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert_string!(parser, *alias, "baz");
        });
    });
}

#[test]
fn test_parse_import_default_and_namespace() {
    // combined default import + namespace import
    let test = TestParser::new(r#"import a, * as b from "foo""#);
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        // default: a
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "a");
        });
        // namespace: * as b
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "b");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_block() {
    let test = TestParser::new(
        "
export { CreateUIMessage, UIMessage }
",
    );
    let mut parser = test.prepare();

    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 2);
        // CreateUIMessage
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "CreateUIMessage");
            assert!(alias.is_none());
        });
        // UIMessage
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "UIMessage");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_with_newline_before_from() {
    let test = TestParser::new("export { A }\nfrom 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    // parse multiline named export with from on the next line
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "A");
            assert!(alias.is_none());
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_namespace_with_newline_before_from() {
    let test = TestParser::new("export *\nfrom 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    // parse multiline namespace export with from on the next line
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_namespace() {
    let test = TestParser::new("export * from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_namespace_alias() {
    let test = TestParser::new("export * as foo from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "foo");
        });
        assert_string!(parser, *target, "foo");
    });
}

/// Recover namespace exports without a `from` target.
#[test]
fn test_recover_export_namespace_without_target() {
    let test =
        TestParser::new("export * as from './module.tspp';\nexport type Recovered = string;");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 2);
    assert_eq!(parser.errors.len(), 1);
}

#[test]
fn test_parse_export_default_from_item() {
    let test = TestParser::new("export default foo");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: None, items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, value: Some(value), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_expression_path!(parser, parser.tree.get(*value), "foo");
        });
    });
}

#[test]
fn test_parse_export_default_from_target() {
    let test = TestParser::new("export { default } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_default_from_target_with_alias_and_items() {
    let test = TestParser::new("export { default as bar, baz as baz, biz } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 3);
        // default as bar
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "bar");
        });
        // baz as baz
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "baz");
            assert_string!(parser, *alias, "baz");
        });
        // biz
        assert_node!(parser.tree, items[2], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "biz");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_default_with_multiple_aliases() {
    let test = TestParser::new("export { default as bar, default as baz } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 2);
        // default as bar
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "bar");
        });
        // default as baz
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "baz");
        });
    });
}

#[test]
fn test_report_export_type_without_binding() {
    // source: export type
    let test = TestParser::new("export type");
    let mut parser = test.prepare();
    let error = parser.parse_export().unwrap_err();

    // type
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_report_type_marked_import_items() {
    // source: import type {} from 'x'
    let test = TestParser::new("import type {} from 'x'");
    let mut parser = test.prepare();
    let error = parser.parse_import().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_report_type_marked_import_namespace() {
    // source: import type * as ns from 'x'
    let test = TestParser::new("import type * as ns from 'x'");
    let mut parser = test.prepare();
    let error = parser.parse_import().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_report_type_marked_import_default() {
    // source: import type A from 'x'
    let test = TestParser::new("import type A from 'x'");
    let mut parser = test.prepare();
    let error = parser.parse_import().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_recover_type_marked_import_item() {
    // source: import { type Foo } from 'x'
    let test = TestParser::new("import { type Foo } from 'x'");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.errors[0].kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(parser.errors[0].range()), "type");
}

#[test]
fn test_report_type_marked_import_default_with_items() {
    // source: import type A, { B } from 'x'
    let test = TestParser::new("import type A, { B } from 'x'");
    let mut parser = test.prepare();
    let error = parser.parse_import().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_parse_import_type_as_default_name_with_items() {
    // source: import type, { B } from 'x'
    let test = TestParser::new("import type, { B } from 'x'");
    let mut parser = test.prepare();
    let import_id = parser.parse_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
    });
}

#[test]
fn test_report_type_marked_export_items() {
    // source: export type { Foo }
    let test = TestParser::new("export type { Foo }");
    let mut parser = test.prepare();
    let error = parser.parse_export().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_report_type_marked_export_star() {
    // source: export type * from 'x'
    let test = TestParser::new("export type * from 'x'");
    let mut parser = test.prepare();
    let error = parser.parse_export().unwrap_err();

    // type
    assert_eq!(error.kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(error.range()), "type");
}

#[test]
fn test_recover_type_marked_export_item() {
    // source: export { type Foo }
    let test = TestParser::new("export { type Foo }");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    assert_eq!(expressions.len(), 1);
    assert_eq!(parser.errors.len(), 1);
    assert_eq!(parser.errors[0].kind(), ParserErrorKind::TypeDependency);
    assert_eq!(parser.range_str(parser.errors[0].range()), "type");
}

#[test]
fn test_report_export_default_enum() {
    // source: export default enum A { X, Y, Z }
    let test = TestParser::new("export default enum A { X, Y, Z }");
    let mut parser = test.prepare();
    let error = parser.parse_export().unwrap_err();

    // enum
    assert_eq!(parser.range_str(error.range()), "enum");
}

#[test]
fn test_parse_export_keyword_name_without_target() {
    // source: export { if }
    let test = TestParser::new("export { if }");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "if");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_keyword_alias_without_target() {
    // source: export { if as foo }
    let test = TestParser::new("export { if as foo }");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "if");
            assert_string!(parser, *alias, "foo");
        });
    });
}

#[test]
fn test_parse_export_keyword_string_alias_without_target() {
    let test = TestParser::new(r#"export { localName as "external-name" }"#);
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "localName");
            assert_string!(parser, *alias, "external-name");
        });
    });
}

#[test]
fn test_parse_export_keyword_literal_alias_without_target() {
    // source: export { true_instance as true, false_instance as false, null_instance as null }
    let source = "export { true_instance as true, false_instance as false, null_instance as null }";
    let test = TestParser::new(source);
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    // export
    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 3);

        // true alias
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "true_instance");
            assert_string!(parser, *alias, "true");
        });

        // false alias
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "false_instance");
            assert_string!(parser, *alias, "false");
        });

        // null alias
        assert_node!(parser.tree, items[2], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "null_instance");
            assert_string!(parser, *alias, "null");
        });
    });
}

#[test]
fn test_parse_export_as_identifier_without_target() {
    // source: export { as }
    let test = TestParser::new("export { as }");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "as");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_type_identifier_without_target() {
    // source: export { type }
    let test = TestParser::new("export { type }");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_named_type_with_keyword_alias_without_target() {
    // source: export { type as if }
    let test = TestParser::new("export { type as if }");
    let mut parser = test.prepare();
    let export_id = parser.parse_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "type");
            assert_string!(parser, *alias, "if");
        });
    });
}

#[test]
fn test_report_export_function_without_name() {
    let test = TestParser::new("export function(option: unknown): void");
    let mut parser = test.prepare();
    let error = parser.parse_export().unwrap_err();

    assert_eq!(parser.range_str(error.range()), "function");
}

#[test]
fn test_parse_root_import_named_binding_from_source() {
    let test = TestParser::new("import {a} from 'a';");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "a");
        let items = import_items(items);
        assert_eq!(items.len(), 1);
    });
}

#[test]
fn test_parse_root_import_default_and_namespace() {
    let test = TestParser::new("import a, * as b from 'a';");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "a");
        let items = import_items(items);
        assert_eq!(items.len(), 2);
    });
}

#[test]
fn test_parse_root_export_named_binding_from_source() {
    let test = TestParser::new("export {a} from 'a';");
    let mut parser = test.prepare();
    let expressions = parser.parse_in_place();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Export { target, items, .. } => {
        assert!(target.is_some());
        assert_eq!(items.len(), 1);
    });
}
