use destack_core::StringId;
use destack_dir::{
    DependencyBinding, DependencyForm, DependencyItem, Expression, ImportAttribute,
    ImportAttributeClauseKind, ImportAttributeValue, LocalNodeId, Name, ScalarLiteral,
};
use destack_source::{LanguageType, NodeSpanList, NodeSpanRegion, NodeSpanType, Span};

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
    let mut test = TestParser::new("import \"destack\"");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // import
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "destack");
    });
}

#[test]
fn test_parse_import_from_expression() {
    let mut test = TestParser::new("import os from 'os'");
    let mut parser = test.prepare();
    let expression_id = parser.eat_expression(parser.flags).unwrap();

    // import os from 'os'
    assert_node!(parser.tree, expression_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
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
    let mut test = TestParser::new("import \"destack.geometry\" with { bar: true }");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // import sample.module with { bar: true }
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, attributes, .. } => {
        // sample.module
        assert_eq!(*form, DependencyForm::Plain);
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "destack.geometry");

        // with { bar: true }
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        let entries = &attributes.attributes;
        assert_eq!(entries.len(), 1);
        assert_string!(parser, entries[0].key.string(), "bar");
        assert_eq!(
            entries[0].value,
            ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true))
        );
    });

    let source = parser.file.text();
    let clause_start = source.find("with").unwrap() as u32;
    let attribute_start = source.find("bar").unwrap() as u32;
    let attribute_end = source.find(" }").unwrap() as u32;

    assert_eq!(
        parser
            .tree
            .get_side_span(import_id, NodeSpanType::Region(NodeSpanRegion::Clause)),
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
    let mut test = TestParser::new("import \"destack.geometry\" with { bar: true");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_eq!(parser.errors.len(), 1);

    assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        assert_eq!(attributes.attributes.len(), 1);
        assert_string!(parser, attributes.attributes[0].key.string(), "bar");
        assert_eq!(
            attributes.attributes[0].value,
            ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true))
        );
    });
}

#[test]
fn test_parse_import_path_with_nested_attributes() {
    let mut test = TestParser::new(
        "import \"destack.geometry\" with { mode: \"json\", options: { eager: true, levels: [1, 2] } }",
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { attributes, .. } => {
        let attributes = attributes.as_ref().expect("expected attributes");
        assert_eq!(attributes.kind, ImportAttributeClauseKind::With);
        assert_eq!(attributes.attributes.len(), 2);

        assert_string!(parser, attributes.attributes[0].key.string(), "mode");
        assert_eq!(
            attributes.attributes[0].value,
            ImportAttributeValue::ScalarLiteral(ScalarLiteral::String(parser.strings.intern("json")))
        );

        assert_string!(parser, attributes.attributes[1].key.string(), "options");
        assert_eq!(
            attributes.attributes[1].value,
            ImportAttributeValue::Object(vec![
                ImportAttribute {
                    key: Name::Identifier(parser.strings.intern("eager")),
                    value: ImportAttributeValue::ScalarLiteral(ScalarLiteral::Boolean(true)),
                },
                ImportAttribute {
                    key: Name::Identifier(parser.strings.intern("levels")),
                    value: ImportAttributeValue::Array(vec![
                        ImportAttributeValue::ScalarLiteral(ScalarLiteral::Integer(1)),
                        ImportAttributeValue::ScalarLiteral(ScalarLiteral::Integer(2)),
                    ]),
                },
            ])
        );
    });
}

#[test]
fn test_parse_import_with_prefix_items() {
    let mut test = TestParser::new("import { Vector2, Vector3 as V3 } from \"ds.geometry\"");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias, .. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Vector2");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem::Binding { form, name: Some(name), alias: Some(alias),.. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Vector3");
            assert_string!(parser, *alias, "V3");
        });
        assert_import_target_string(&parser, *target, "ds.geometry");
    });
}

#[test]
fn test_parse_import_as_alias() {
    let mut test = TestParser::new(r#"import * as geom from "ds/geometry""#);
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // import * as geom from ds.geometry
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
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
    let mut test = TestParser::new("import { A }\nfrom 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // parse multiline named import with from on the next line
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "A");
            assert!(alias.is_none());
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_before_from() {
    let mut test = TestParser::new(
        "import HeaderNavigationButton
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // parse multiline default import with from on the next line
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_eq!(*form, None);
            assert_string!(parser, *alias, "HeaderNavigationButton");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_block_items() {
    let mut test = TestParser::new(
        "import Default,
{ type Item }
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // parse multiline default plus named imports
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 2);

        // default binding
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "Default");
        });

        // type named binding
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "Item");
            assert!(alias.is_none());
        });

        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_default_with_newline_comment_before_from() {
    let mut test = TestParser::new(
        "import BreakoutRooms
// @ts-ignore
from 'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // parse default import with comment between binding and from
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "BreakoutRooms");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_with_newline_after_from() {
    let mut test = TestParser::new(
        "import { goBack } from
'foo'",
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    // parse import target on the next line after from
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "goBack");
            assert!(alias.is_none());
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_with_type() {
    let mut test = TestParser::new(
        "
import {
  StructuredObject,
  type StructuredObjectOptions,
} from './lib/object.ng';
",
    );
    let mut parser = test.prepare();

    let import_id = parser.eat_import().unwrap();
    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_import_target_string(&parser, *target, "./lib/object.ng");

        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias,.. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "StructuredObject");
            assert!(alias.is_none());
        });
        assert_node!(parser.tree, items[1], DependencyItem::Binding { form, name: Some(name), alias,.. } => {
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "StructuredObjectOptions");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_items_without_separator_spaces() {
    let mut test = TestParser::new("import {foo,bar,baz} from 'module'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

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
    let mut test = TestParser::new("import Default, { type Item } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        // Default
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "Default");
        });
        // { type Item }
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias: None,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "Item");
        });
        // `foo`
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_type_identifier_name() {
    // treat type as a value name in named imports
    let mut test = TestParser::new("import { type } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias, .. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_type_as_default_name() {
    let mut test = TestParser::new("import type from './a'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

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
fn test_parse_import_type_empty_block() {
    let mut test = TestParser::new("import type {} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let _items = assert_empty_import_shell(items);
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_named_alias_after_newline_comment() {
    let mut test =
        TestParser::new("import {\n  a\n  // keep alias on next line\n  as b\n} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

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
    let mut test = TestParser::new("import // keep target on next line\n'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        assert_bare_import(items);
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_block_with_newline_after_comment() {
    let mut test = TestParser::new("import // keep binding on next line\n{} from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let _items = assert_empty_import_shell(items);
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_import_type_string_specifier() {
    let mut test = TestParser::new(r#"import { type "string" as foo } from "foo""#);
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*form, Some(DependencyForm::Type));
            assert!(matches!(name, Name::String(_)));
            assert_string!(parser, name.string(), "string");
            assert_string!(parser, *alias, "foo");
        });
    });
}

#[test]
fn test_parse_import_type_as_value_alias() {
    // treat type as a value name when followed by as as
    let mut test = TestParser::new("import { type as as } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "type");
            assert_string!(parser, *alias, "as");
        });
    });
}

#[test]
fn test_parse_import_type_only_named_as() {
    // treat type as a modifier when followed by as then close brace
    let mut test = TestParser::new("import { type as } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias, .. } => {
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "as");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_type_in_import_type_recovers_error_item() {
    // preserve one broken item inside import type blocks
    let mut test = TestParser::new("import type { type Foo } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { form, items, target, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Error);
    });
}

#[test]
fn test_parse_import_block_recovers_broken_alias_item() {
    // preserve valid siblings after one broken import item
    let mut test = TestParser::new("import { Foo as, Bar } from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Error);
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_block_recovers_missing_close_before_from() {
    // preserve the import clause when `}` is omitted before from
    let mut test = TestParser::new("import { Foo from 'foo'");
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { items, target, .. } => {
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_import_target_string(&parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Foo");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_import_block_keeps_statement_owner_in_missing_close_gap() {
    // keep the import expression enclosing the whitespace gap before `from`
    let source = "import { Widget,  from \"./types.ds\";";
    let cursor = source.find("  from").unwrap() as u32 + 1;
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let roots = parser.parse();
    let root_id = roots[0];

    assert_node!(parser.tree, root_id, Expression::Import { items, target, .. } => {
            let items = import_items(items);
            assert_eq!(items.len(), 1);
            assert_import_target_string(&parser, *target, "./types.ds");
            assert_node!(parser.tree, items[0], DependencyItem::Binding { name: Some(name), .. } => {
                assert_string!(parser, name.string(), "Widget");
            });
    });

    let enclosing = parser.tree.source_map.get_enclosing_spans(cursor, cursor);
    assert!(enclosing.iter().any(|span| span.idx == root_id.id));
}

#[test]
fn test_parse_import_keeps_target_main_span_for_unterminated_path() {
    // keep the import expression and target main span around the trailing path byte
    let source = "import { } from \"./u";
    let cursor = source.len() as u32;
    let probe = cursor.saturating_sub(1);
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let roots = parser.parse();
    let root_id = roots[0];

    assert_node!(parser.tree, root_id, Expression::Import { target, .. } => {
            assert_import_target_string(&parser, *target, "./u");
    });

    let enclosing = parser.tree.source_map.get_enclosing_spans(probe, probe);
    assert!(enclosing.iter().any(|span| span.idx == root_id.id));

    let main_span = parser.tree.source_map.get_main(root_id.id).unwrap();
    assert!(main_span.contains(probe));
}

#[test]
fn test_parse_export_block_recovers_broken_alias_item() {
    // preserve valid siblings after one broken export item
    let mut test = TestParser::new("export { Foo as, Bar } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { items, target: Some(target), .. } => {
        assert_eq!(items.len(), 2);
        assert_string!(parser, *target, "foo");
        assert_node!(parser.tree, items[0], DependencyItem::Error);
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Bar");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_type_identifier_name() {
    // treat type as a value name in named exports
    let mut test = TestParser::new("export { type } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { items, .. } => {
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { form, name: Some(name), alias, .. } => {
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_clause_after_comment_newline_keyword() {
    let mut test = TestParser::new_with_language("export //comment\n{}", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target, items, .. } => {
        assert!(target.is_none());
        assert!(items.is_empty());
    });
}

#[test]
fn test_parse_export_specifier_alias_after_comment_newline() {
    let mut test = TestParser::new_with_language(
        "export {\n  bar as // comment\n  baz,\n} from 'foo'",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { target: Some(target), items, .. } => {
        assert_string!(parser, *target, "foo");
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert_string!(parser, *alias, "baz");
        });
    });
}

#[test]
fn test_parse_import_specifier_alias_after_comment_newline() {
    let mut test = TestParser::new_with_language(
        "import {\n  bar as // comment\n  baz,\n} from 'foo'",
        LanguageType::JavaScript,
    );
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "foo");
        let items = import_items(items);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_string!(parser, name.string(), "bar");
            assert_string!(parser, *alias, "baz");
        });
    });
}

#[test]
fn test_parse_import_default_and_namespace() {
    // combined default import + namespace import
    let mut test = TestParser::new(r#"import a, * as b from "foo""#);
    let mut parser = test.prepare();
    let import_id = parser.eat_import().unwrap();

    assert_node!(parser.tree, import_id, Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        let items = import_items(items);
        assert_eq!(items.len(), 2);
        // default: a
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form: None, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_string!(parser, *alias, "a");
        });
        // namespace: * as b
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form: None, name: None, alias: Some(alias),.. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "b");
        });
        assert_import_target_string(&parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_block() {
    let mut test = TestParser::new(
        "
export type { CreateUIMessage, UIMessage }
",
    );
    let mut parser = test.prepare();

    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        assert!(target.is_none());
        assert_eq!(items.len(), 2);
        // CreateUIMessage
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "CreateUIMessage");
            assert!(alias.is_none());
        });
        // UIMessage
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias,.. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "UIMessage");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_with_newline_before_from() {
    let mut test = TestParser::new("export { A }\nfrom 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    // parse multiline named export with from on the next line
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "A");
            assert!(alias.is_none());
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_namespace_with_newline_before_from() {
    let mut test = TestParser::new("export *\nfrom 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    // parse multiline namespace export with from on the next line
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_namespace() {
    let mut test = TestParser::new("export * from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_type_with_namespace() {
    let mut test = TestParser::new("export type * from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_type_namespace_string_alias() {
    let mut test = TestParser::new(r#"export type * as "ns2" from 'foo'"#);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "ns2");
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_type_item_reexport() {
    let mut test = TestParser::new("export { type Options } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "Options");
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_type_reexport_block() {
    let mut test = TestParser::new("export type { Options } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: None, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "Options");
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_with_namespace_alias() {
    let mut test = TestParser::new("export * as foo from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Namespace);
            assert_string!(parser, *alias, "foo");
        });
        assert_string!(parser, *target, "foo");
    });
}

#[test]
fn test_parse_export_default_from_item() {
    let mut test = TestParser::new("export default foo");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: None, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, name: None, alias: None, value: Some(value), .. } => {
            assert_eq!(*binding, DependencyBinding::Default);
            assert_expression_path!(parser, parser.tree.get(*value), "foo");
        });
    });
}

#[test]
fn test_parse_export_default_from_target() {
    let mut test = TestParser::new("export { default } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
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
    let mut test = TestParser::new("export { default as bar, baz as baz, biz } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
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
    let mut test = TestParser::new("export { default as bar, default as baz } from 'foo'");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();
    assert_node!(parser.tree, export_id, Expression::Export { form, target: Some(target), items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
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
fn test_reject_export_type_without_binding() {
    // source: export type
    let source = "export type";
    let mut test = TestParser::new_with_language("export type", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let error = parser.eat_export().unwrap_err();

    // eof
    assert_eq!(parser.get_span_str(error.leaf_span()), "");
    assert_eq!(error.leaf_span().start, source.len() as u32);
}

#[test]
fn test_reject_export_default_enum() {
    // source: export default enum A { X, Y, Z }
    let mut test = TestParser::new("export default enum A { X, Y, Z }");
    let mut parser = test.prepare();
    let error = parser.eat_export().unwrap_err();

    // enum
    assert_eq!(parser.get_span_str(error.leaf_span()), "enum");
}

#[test]
fn test_parse_export_keyword_name_without_target() {
    // source: export { if }
    let mut test = TestParser::new("export { if }");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "if");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_keyword_alias_without_target() {
    // source: export { if as foo }
    let mut test = TestParser::new("export { if as foo }");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "if");
            assert_string!(parser, *alias, "foo");
        });
    });
}

#[test]
fn test_parse_export_keyword_string_alias_without_target() {
    let mut test = TestParser::new(r#"export { localName as "external-name" }"#);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "localName");
            assert_string!(parser, *alias, "external-name");
        });
    });
}

#[test]
fn test_parse_export_keyword_literal_alias_without_target() {
    // source: export { true_instance as true, false_instance as false, null_instance as null }
    let source = "export { true_instance as true, false_instance as false, null_instance as null }";
    let mut test = TestParser::new(source);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    // export
    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 3);

        // true alias
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "true_instance");
            assert_string!(parser, *alias, "true");
        });

        // false alias
        assert_node!(parser.tree, items[1], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "false_instance");
            assert_string!(parser, *alias, "false");
        });

        // null alias
        assert_node!(parser.tree, items[2], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "null_instance");
            assert_string!(parser, *alias, "null");
        });
    });
}

#[test]
fn test_parse_export_as_identifier_without_target() {
    // source: export { as }
    let mut test = TestParser::new("export { as }");
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "as");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_type_identifier_without_target() {
    // source: export { type }
    let mut test = TestParser::new_with_language("export { type }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias, .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "type");
            assert!(alias.is_none());
        });
    });
}

#[test]
fn test_parse_export_named_type_with_keyword_alias_without_target() {
    // source: export { type as if }
    let mut test = TestParser::new_with_language("export { type as if }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, None);
            assert_string!(parser, name.string(), "type");
            assert_string!(parser, *alias, "if");
        });
    });
}

#[test]
fn test_parse_export_type_only_as_as_keyword_alias_without_target() {
    // source: export { type as as if }
    let mut test =
        TestParser::new_with_language("export { type as as if }", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let export_id = parser.eat_export().unwrap();

    assert_node!(parser.tree, export_id, Expression::Export { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Plain);
        assert!(target.is_none());
        assert_eq!(items.len(), 1);
        assert_node!(parser.tree, items[0], DependencyItem::Binding { binding, form, name: Some(name), alias: Some(alias), .. } => {
            assert_eq!(*binding, DependencyBinding::Named);
            assert_eq!(*form, Some(DependencyForm::Type));
            assert_string!(parser, name.string(), "as");
            assert_string!(parser, *alias, "if");
        });
    });
}

#[test]
fn test_reject_export_function_without_name() {
    let mut test = TestParser::new_with_language(
        "export function(option: any): void",
        LanguageType::TypeScript,
    );
    let mut parser = test.prepare();
    let result = parser.eat_export();
    assert!(result.is_err());
}

#[test]
fn test_parse_root_import_named_binding_from_source() {
    let mut test = TestParser::new_with_language("import {a} from 'a';", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

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
    let mut test =
        TestParser::new_with_language("import a, * as b from 'a';", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Import { target, items, .. } => {
        assert_import_target_string(&parser, *target, "a");
        let items = import_items(items);
        assert_eq!(items.len(), 2);
    });
}

#[test]
fn test_parse_root_empty_type_import() {
    let mut test =
        TestParser::new_with_language("import type {} from 'a';", LanguageType::TypeScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Import { form, target, items, .. } => {
        assert_eq!(*form, DependencyForm::Type);
        assert_import_target_string(&parser, *target, "a");
        let _items = assert_empty_import_shell(items);
    });
}

#[test]
fn test_parse_root_export_named_binding_from_source() {
    let mut test = TestParser::new_with_language("export {a} from 'a';", LanguageType::JavaScript);
    let mut parser = test.prepare();
    let expressions = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(expressions.len(), 1);
    assert_node!(parser.tree, expressions[0], Expression::Export { target, items, .. } => {
        assert!(target.is_some());
        assert_eq!(items.len(), 1);
    });
}
