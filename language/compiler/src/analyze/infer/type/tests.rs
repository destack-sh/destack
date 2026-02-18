use crate::TestProgram;
use destack_dir::{Expression, StaticKey, SymbolSpace};

/// Detect export inference cycles from module dependencies.
#[test]
fn test_export_inference_cycle_detected_from_imports() {
    let test = TestProgram::memory_sequential();
    let a_module_id = test.add_module(
        "a.ts",
        r#"
import { y } from "./b";

export const x = y;
"#,
    );
    let b_module_id = test.add_module(
        "b.ts",
        r#"
import { x } from "./a";

export const y = x;
"#,
    );

    test.resolve_module(a_module_id);
    test.resolve_module(b_module_id);
    test.compile();

    let profile = test.default_profile_id(a_module_id);
    let module = test.program.modules.get(a_module_id);
    let module = module.read();
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let exported_symbols = dir.exported_symbols.read();

    let x_name = test.program.strings.intern("x");
    let x_key = StaticKey::Name(x_name);
    assert!(
        exported_symbols.contains_key(&(SymbolSpace::Value, x_key)),
        "expected export table to include x as a value export"
    );

    let declarator_id = crate::expect_let_declarator_by_name(&dir.roots, &tree, x_name);
    let declarator = tree.get(declarator_id);
    let value_id = declarator.value.expect("expected initializer for export x");
    let Expression::ModuleReference { target_symbol, .. } = tree.get(value_id) else {
        panic!("expected module reference for export initializer");
    };
    let symbol_entry = symbols.get_symbol(target_symbol.local_id);
    assert!(
        symbol_entry.target_symbol.is_some(),
        "expected import binding to resolve to a target symbol"
    );

    let y_symbol = test
        .resolve_to_symbol("b.ts", "y")
        .expect("expected y symbol in b.ts");
    let has_declared_type = test
        .compiler
        .remote_symbol_has_declared_value_type(profile, y_symbol);
    assert!(
        !has_declared_type,
        "expected unannotated export y to have no declared value type"
    );

    let has_cycle = test
        .compiler
        .export_inference_has_cycle(a_module_id, profile, b_module_id)
        .expect("cycle detection should not error");

    assert!(
        has_cycle,
        "expected export inference cycle for a.ts <-> b.ts"
    );
}
