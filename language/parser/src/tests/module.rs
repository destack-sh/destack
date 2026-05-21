use destack_dir::{Declaration, Decorator, DecoratorPosition, Expression, ModuleDeclaration};
use destack_source::LanguageType;

use crate::{TestParser, assert_expression_path, assert_node};

#[test]
fn test_parse_module_newline_as_identifiers() {
    let mut test = TestParser::new_with_language("module\nFoo\n{}", LanguageType::TypeScript);
    let mut parser = test.prepare();

    let expressions = parser.parse();
    assert_eq!(expressions.len(), 3);

    let module_id = expressions[0];
    assert_expression_path!(parser, parser.tree.get(module_id), "module");

    let name_id = expressions[1];
    assert_expression_path!(parser, parser.tree.get(name_id), "Foo");

    let object_id = expressions[2];
    assert_node!(parser.tree, object_id, Expression::Block(_) => {});
}

#[test]
fn test_parse_module_declaration() {
    let mut test = TestParser::new(
        r###"
module {
    const tree = HtmlTree
}
"###,
    );
    let mut parser = test.prepare();

    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Module(ModuleDeclaration { expressions }) => {
            assert_eq!(expressions.len(), 1);
        });
    });
}

#[test]
fn test_parse_module_declaration_decorators() {
    let mut test = TestParser::new(
        r###"
@noManaged
@noHeap
module {}
"###,
    );
    let mut parser = test.prepare();

    let expressions = parser.parse();
    assert_eq!(expressions.len(), 1);

    let expression_id = expressions[0];
    assert_node!(parser.tree, expression_id, Expression::Declaration(declaration_id) => {
        assert_node!(parser.tree, *declaration_id, Declaration::Module(ModuleDeclaration { expressions }) => {
            assert!(expressions.is_empty());

            let decorators = parser.tree.get_decorators(declaration_id.id);
            assert_eq!(decorators.len(), 2);

            assert_node!(parser.tree, decorators[0], Decorator { expression, position } => {
                assert_eq!(*position, DecoratorPosition::BlockPrefix);
                assert_expression_path!(parser, parser.tree.get(*expression), "noManaged");
            });

            assert_node!(parser.tree, decorators[1], Decorator { expression, position } => {
                assert_eq!(*position, DecoratorPosition::BlockPrefix);
                assert_expression_path!(parser, parser.tree.get(*expression), "noHeap");
            });
        });
    });
}
