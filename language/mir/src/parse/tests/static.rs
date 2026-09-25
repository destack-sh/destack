use tspp_core::StringId;

use crate::{Function, Symbol};

use super::TestParser;

/// Imported instances retain the symbol derived from their static arguments.
#[test]
fn test_parse_imported_instance_symbol() {
    let source = r#"
external function take<4>(): void
"#;
    let (tree, _) = TestParser::new(source).parse();
    let (_, function) = tree
        .iter_nodes::<Function>()
        .next()
        .expect("fixture should contain one function");
    let base = Symbol::named(crate::TEST_MODULE, StringId::for_text("take"));
    let expected = base.instantiate(&function.arguments, &tree);

    assert_eq!(function.symbol, expected);
}
