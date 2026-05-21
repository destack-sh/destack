use destack_dir::{Declaration, Expression, GlobalDeclaration};
use destack_source::LanguageType;

use crate::{TestParser, assert_node};

#[test]
fn test_parse_declare_global_block() {
    let mut test = TestParser::new(
        r###"
declare global {
    interface Foo {
        bar(): boolean
    }
}
"###,
    );
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert_eq!(*is_ambient, true);
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_global_block_without_declare() {
    let mut test = TestParser::new_with_language(
        r###"
global {
    interface Foo { }
}
"###,
        LanguageType::TypeScriptDeclaration,
    );
    let mut parser = test.prepare();

    let expression_id = parser.eat_expression(parser.flags).unwrap();
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert_eq!(*is_ambient, true);
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_global_declaration() {
    let mut test = TestParser::new(
        r###"
global {
    let process: Process
}
"###,
    );
    let mut parser = test.prepare();

    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Global(GlobalDeclaration { is_ambient, expressions, .. }) => {
            assert_eq!(*is_ambient, false);
            assert_eq!(expressions.len(), 1);
        });
    });
}
