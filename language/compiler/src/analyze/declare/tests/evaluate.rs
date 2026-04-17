use super::{ModuleId, TestProgram};
use crate::analyze::common::{AnalyzeIndex, SymbolTypeView, TypeContext};
use crate::analyze::declare::{StaticConstantResolutionMode, TypeMemberResolution};
use crate::{AnalyzeError, CompilePhase, run_to_completion};
use destack_dir::{
    Declarator, Expression, FloatType, LocalScopeMark, LocalSymbolId, LocalTypeId, Pattern,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, Type, TypeLiteral,
    TypeTable, TypeUnaryOperator,
};
use std::sync::Arc;

/// Assert a fixed-array count resolves to either an integer literal or a named symbol.
fn assert_count_matches_integer_or_symbol_name(
    ctx: SymbolTypeView<'_>,
    count: LocalTypeId,
    expected_integer: i64,
    expected_symbol_name: StaticKey,
) {
    let mut type_id = ctx.types.unwrap_value_type_id(count);
    for _ in 0..16 {
        match ctx.types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            } => {
                assert_eq!(*value, expected_integer);
                return;
            }
            Type::Value { value } => type_id = *value,
            Type::Reference { symbol, .. } => {
                let symbol_key = ctx.symbols.get_symbol(symbol.local_id).key;
                assert_eq!(symbol_key, Some(expected_symbol_name));
                return;
            }
            other => panic!("expected integer literal or reference count type, got {other:?}"),
        }
    }

    panic!("expected integer literal or reference count type within unwrap steps")
}

/// Assert a fixed-array count resolves to one integer literal value.
fn assert_count_resolves_to_integer(count: LocalTypeId, expected_integer: i64, types: &TypeTable) {
    let mut type_id = types.unwrap_value_type_id(count);
    for _ in 0..24 {
        match types.get_type(type_id) {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(value)),
            } => {
                assert_eq!(*value, expected_integer);
                return;
            }
            Type::Value { value } => {
                type_id = *value;
            }
            Type::Unary {
                operator: TypeUnaryOperator::AsComptime,
                right,
            } => {
                type_id = *right;
            }
            Type::Reference { symbol, .. } => {
                if let Some(value_type_id) = types.get_value_type_id(*symbol) {
                    type_id = value_type_id;
                } else if let Some(alias_target_type_id) = types.get_alias_target_type_id(*symbol) {
                    type_id = alias_target_type_id;
                } else {
                    panic!(
                        "expected count reference to resolve to an integer literal, got unresolved symbol {symbol:?}"
                    );
                }
            }
            other => panic!("expected count integer literal chain, got {other:?}"),
        }
    }

    panic!("expected integer literal count within unwrap steps")
}

/// Resolve one declared namespace annotation type from exact declared artifact inputs.
fn resolve_declared_namespace_annotation_type(
    test: &TestProgram,
    module_id: ModuleId,
    name: &str,
) -> (TypeTable, LocalTypeId) {
    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir_declared_data(module_id, profile);
    let tree = Arc::unwrap_or_clone(dir.tree);
    let symbols = Arc::unwrap_or_clone(dir.symbols);
    let mut types = Arc::unwrap_or_clone(dir.types);
    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);

    let value_symbol = symbols
        .find_active_symbol_up_to(
            namespace_scope,
            StaticKey::Name(test.program.strings.intern(name)),
            LocalScopeMark::end(),
        )
        .map(|symbol: LocalSymbolId| symbol.into_global(module_id))
        .expect("expected namespace symbol");
    let value_pattern_id = symbols
        .get_symbol(value_symbol.local_id)
        .primary_declaration
        .expect("expected primary declaration for namespace symbol")
        .local_id
        .try_into_typed::<Pattern>()
        .expect("expected pattern declaration for namespace symbol");
    let value_declarator_id = tree
        .get_parent(value_pattern_id.id)
        .expect("expected parent declarator for namespace pattern")
        .into_typed::<Declarator>();
    let value_declarator = tree.get(value_declarator_id);
    let value_annotation_id = value_declarator
        .ty
        .expect("expected type annotation for namespace symbol");

    let options = test.analyze_context_options_for_module(module.id);
    let compiler_context = test.context();
    let mut type_context = TypeContext::new(
        &compiler_context,
        module,
        profile,
        &options,
        &tree,
        &symbols,
        &mut types,
        AnalyzeIndex::default(),
    );
    let value_type_id = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.resolve_declared_type_expression(
                &mut type_context,
                value_annotation_id,
                true,
                true,
            )
        },
    );
    let value_type_id = value_type_id.expect("expected declared namespace annotation type");

    (types, value_type_id)
}

#[test]
fn test_analyze_evaluate_type_on_let_expression() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source("test.ds", "declare let x: number");
    let view = test.declare_view(module_id);

    let tree = view.tree();
    let types = view.types();
    let let_expr_id = view.root_expression_id(0);
    let let_expression = tree.get(let_expr_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();

    let let_ty = types
        .get_declared_type(declarator_id.into_global(module_id).into())
        .unwrap();

    assert_eq!(
        *let_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

#[test]
fn test_analyze_evaluate_type_on_let_expression_int() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source("test.ds", "declare let x: int");
    let view = test.declare_view(module_id);

    let tree = view.tree();
    let types = view.types();
    let let_expr_id = view.root_expression_id(0);
    let let_expression = tree.get(let_expr_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();

    let let_ty = types
        .get_declared_type(declarator_id.into_global(module_id).into())
        .unwrap();

    // int resolves to Arbitrary { width: 32, is_signed: true } which is semantically Int32
    assert!(matches!(
        let_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(int_type))
        } if int_type.width() == Some(32) && int_type.is_signed()
    ));
}

#[test]
fn test_recursive_type_alias_reports_recursion() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Loop<T> = Loop<T>;

declare let value: Loop<number>;
"#,
    );

    test.declare_module(module_id);
    test.check_has_diagnostic("EA121");
}

#[test]
fn test_mapped_recursive_type_alias_reports_recursion() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
type Remap<T> = { [K in keyof T]: Remap<T[K]> };

declare let value: Remap<{ name: string }>;
"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA121");
}

#[test]
fn test_cross_module_recursive_type_alias_reports_recursion() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import type { B } from "./b";

export type A<T> = B<T>;
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import type { A } from "./a";

export type B<T> = A<T>;
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import type { A } from "./a";

declare let value: A<number>;
"#,
    );

    test.declare_module(module_id);
    test.check_has_diagnostic("EA121");
}

#[test]
fn test_missing_type_alias_rhs_stays_import_only() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
type T =
"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_no_diagnostics_for_phases(&[CompilePhase::Resolve, CompilePhase::Analyze]);
}

#[test]
fn test_missing_type_member_name_stays_import_only() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ts",
        r#"
interface Foo {
    value: string;
}

type T = Foo.
"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_no_diagnostics_for_phases(&[CompilePhase::Resolve, CompilePhase::Analyze]);

    let view = test.declare_view(module_id);
    let _symbol = view.expect_namespace_symbol("T");
}

#[test]
fn test_type_index_integer_literal_reports_missing_property_and_keeps_index_access_when_not_admissible()
 {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "declare const value: { x: string }[5];");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA202");
    let (types, value_type_id) =
        resolve_declared_namespace_annotation_type(&test, module_id, "value");

    let Type::Index { left, index } = types.get_type(value_type_id) else {
        panic!(
            "expected indexed-access type for non-admissible numeric index, got {:?}",
            types.get_type(value_type_id)
        );
    };

    let index_value = test
        .compiler
        .integer_literal_value_for_type_id(*index, &types)
        .expect("expected integer literal index");
    assert_eq!(index_value, 5, "expected indexed-access key of 5");

    let Type::Object { fields, .. } = types.get_type(*left) else {
        panic!(
            "expected indexed-access receiver to stay as the object type, got {:?}",
            types.get_type(*left)
        );
    };
    assert!(
        fields
            .iter()
            .any(|field| field.key == StaticKey::Name(test.program.strings.intern("x"))),
        "expected receiver object field `x` to be preserved"
    );
}

#[test]
fn test_type_index_integer_literal_uses_index_access_when_index_is_admissible() {
    let test = TestProgram::memory_sequential();
    let module_id = test
        .analyze_declare_module_with_source("test.ds", "declare const value: [string, number][0];");
    let (types, value_type_id) =
        resolve_declared_namespace_annotation_type(&test, module_id, "value");
    let value_type = types.get_type(value_type_id);

    assert!(
        !matches!(value_type, Type::ArraySized { .. }),
        "expected tuple index access to remain indexed access semantics, got {value_type:?}"
    );
    assert!(
        matches!(value_type, Type::Index { .. }),
        "expected tuple index access to preserve index type semantics, got {value_type:?}"
    );
}

#[test]
fn test_type_index_as_comptime_forces_fixed_array_construction() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        "declare const value: [string, number][0 as comptime];",
    );
    let (types, value_type_id) =
        resolve_declared_namespace_annotation_type(&test, module_id, "value");

    let Type::ArraySized { element, count, .. } = types.get_type(value_type_id) else {
        panic!(
            "expected explicit `as comptime` to force fixed-size array construction, got {:?}",
            types.get_type(value_type_id)
        );
    };

    assert_count_resolves_to_integer(*count, 0, &types);

    assert!(
        matches!(types.get_type(*element), Type::Tuple { .. }),
        "expected fixed-size element to be the tuple receiver type, got {:?}",
        types.get_type(*element)
    );
}

/// Preserve nested fixed-size array literals in type positions.
#[test]
fn test_nested_fixed_array_literals_in_type_position() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
declare const grid: float32[16][16];
"#,
    );
    let view = test.declare_view(module_id);

    let tree = view.tree();
    let types = view.types();
    let grid_type_id = view.expect_namespace_value_type_id("grid");

    let Type::ArraySized {
        element: outer_element,
        ..
    } = types.get_type(grid_type_id)
    else {
        panic!(
            "expected outer fixed array type, got {:?}",
            types.get_type(grid_type_id)
        );
    };

    let Type::ArraySized { .. } = types.get_type(*outer_element) else {
        panic!(
            "expected inner fixed array type, got {:?}",
            types.get_type(*outer_element)
        );
    };

    // ensure the type expression tree remains addressable for diagnostics
    let let_expression_id = view.root_expression_id(0);
    let Expression::Let { declarators, .. } = tree.get(let_expression_id) else {
        panic!("expected let declaration");
    };
    let declarator_id = declarators[0];
    let declared_type_id = types
        .get_declared_type_id(declarator_id.into_global_any(module_id))
        .expect("expected declared type id for grid");
    let declared_type = types.get_type(declared_type_id);
    assert!(
        !declared_type.is_unevaluated(),
        "expected declared type to be evaluated, got {declared_type:?}"
    );
}

/// Materialize interface associated comptime members in projected alias counts.
#[test]
fn test_interface_associated_alias_projection_materializes_comptime_counts() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
interface PartitionedStore<Row> {
comptime const SegmentBytes: number;
type Segment = Row[this.SegmentBytes];
}

class AuditStore implements PartitionedStore<string> {
comptime const SegmentBytes: number = 1024;
}

declare const segment: AuditStore.Segment;
"#,
    );

    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir_data(module_id, profile);
    let symbols = &dir.symbols;
    let types = &dir.types;

    let segment_key = StaticKey::Name(test.program.strings.intern("segment"));
    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
    let segment_symbol = symbols
        .find_active_symbol_up_to(namespace_scope, segment_key, LocalScopeMark::end())
        .map(|symbol| symbol.into_global(module.id))
        .expect("expected segment symbol");

    let segment_type_id = types
        .get_value_type_id(segment_symbol)
        .expect("expected segment value type");

    // shape
    let count = match types.get_type(segment_type_id) {
        Type::ArraySized { count, .. } => *count,
        other => panic!("expected fixed-size segment projection, got {other:?}"),
    };

    // count
    assert_count_matches_integer_or_symbol_name(
        SymbolTypeView::new(&test.context(), module, profile, &dir.symbols, types),
        count,
        1024,
        StaticKey::Name(test.program.strings.intern("SegmentBytes")),
    );
}

/// Resolve class associated type and associated comptime member name collisions by value space.
#[test]
fn test_class_associated_alias_projection_materializes_colliding_comptime_counts() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
class NamedWidth<Row> {
type Width = Row;
comptime const Width: number = 8;
type Buffer = uint8[this.Width];
}

declare const buffer: NamedWidth<string>.Buffer;
"#,
    );
    let view = test.declare_view(module_id);
    let types = view.types();
    let buffer_type_id = view.expect_namespace_value_type_id("buffer");

    let Type::ArraySized { count, .. } = types.get_type(buffer_type_id) else {
        panic!(
            "expected fixed-size associated alias projection, got {:?}",
            types.get_type(buffer_type_id)
        );
    };

    let count_value = test
        .compiler
        .integer_literal_value_for_type_id(*count, types);
    assert_eq!(
        count_value,
        Some(8),
        "expected colliding associated comptime projection count to resolve to 8: count={:?}",
        types.get_type(*count),
    );
}

/// Publish declared projection dependencies for associated comptime members.
#[test]
fn test_collect_associated_comptime_projection_dependencies() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
class ProjectionPlan<Row> {
comptime const Scalar: number = 1;
comptime const Dependent: number = ProjectionPlan<Row>.Scalar;
}
"#,
    );

    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir_data(module_id, profile);
    let tree = &dir.tree;
    let symbols = &dir.symbols;
    let types = &dir.types;

    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
    let owner_key = StaticKey::Name(test.program.strings.intern("ProjectionPlan"));
    let owner_symbol = symbols
        .find_active_symbol_up_to(namespace_scope, owner_key, LocalScopeMark::end())
        .map(|symbol| symbol.into_global(module.id))
        .expect("expected ProjectionPlan symbol");

    let scalar_key = StaticKey::Name(test.program.strings.intern("Scalar"));
    let scalar_symbol = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<Option<_>, AnalyzeError>(compiler.query_static_member_symbol(
                _context.revision(),
                module,
                profile,
                owner_symbol,
                scalar_key,
                tree,
                symbols,
            ))
        },
    )
    .expect("failed to query Scalar member symbol");
    let scalar_symbol = scalar_symbol.expect("expected Scalar member symbol");

    let dependent_key = StaticKey::Name(test.program.strings.intern("Dependent"));
    let dependent_symbol = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<Option<_>, AnalyzeError>(compiler.query_static_member_symbol(
                _context.revision(),
                module,
                profile,
                owner_symbol,
                dependent_key,
                tree,
                symbols,
            ))
        },
    )
    .expect("failed to query Dependent member symbol");
    let dependent_symbol = dependent_symbol.expect("expected Dependent member symbol");

    assert!(
        !types.symbol_has_associated_comptime_projection_dependencies(scalar_symbol),
        "expected Scalar to stay non projection dependent"
    );
    assert!(
        types.symbol_has_associated_comptime_projection_dependencies(dependent_symbol),
        "expected Dependent to be marked projection dependent"
    );
}

/// Materialize nested associated comptime alias counts for interface defaults.
#[test]
fn test_interface_associated_alias_projection_materializes_nested_comptime_counts() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
interface KernelProfile<T> {
comptime const LaneWidth: int = T extends float32 ? 16 : 8;
comptime const TileRows: int = this.LaneWidth * 2;

type Tile = T[this.TileRows][this.LaneWidth];
}

class F32Kernel implements KernelProfile<float32> {}

declare const tile: F32Kernel.Tile;
"#,
    );
    let view = test.declare_view(module_id);
    let types = view.types();
    let tile_type_id = view.expect_namespace_value_type_id("tile");

    let Type::ArraySized {
        element: outer_element,
        count: outer_count,
        ..
    } = types.get_type(tile_type_id)
    else {
        panic!(
            "expected outer fixed-size tile projection, got {:?}",
            types.get_type(tile_type_id)
        );
    };
    let Type::ArraySized {
        count: inner_count, ..
    } = types.get_type(*outer_element)
    else {
        panic!(
            "expected inner fixed-size tile projection, got {:?}",
            types.get_type(*outer_element)
        );
    };

    let outer_literal = test
        .compiler
        .integer_literal_value_for_type_id(*outer_count, types);
    let inner_literal = test
        .compiler
        .integer_literal_value_for_type_id(*inner_count, types);

    assert_eq!(
        outer_literal,
        Some(16),
        "expected outer tile count to resolve to 16"
    );
    assert_eq!(
        inner_literal,
        Some(32),
        "expected inner tile count to resolve to 32: inner_count={:?}",
        types.get_type(*inner_count),
    );
}

/// Materialize associated comptime arguments through generic interface alias roots.
#[test]
fn test_materialize_associated_comptime_arguments_through_generic_interface_alias_roots() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
newtype Vector<T, comptime N: int> = T;

interface KernelProfile<T> {
comptime const LaneWidth: int = T extends float32 ? 16 : 8;
type Lane = Vector<T, this.LaneWidth>;
}

class F32Kernel implements KernelProfile<float32> {}

declare const lane: F32Kernel.Lane;
"#,
    );
    let view = test.declare_view(module_id);
    let types = view.types();
    let lane_type_id = view.expect_namespace_value_type_id("lane");

    let Type::Reference {
        symbol: _,
        generic_arguments: Some(arguments),
    } = types.get_type(lane_type_id)
    else {
        panic!(
            "expected projected vector reference, got {:?}",
            types.get_type(lane_type_id)
        );
    };
    let [
        StaticArgument::Evaluated {
            value:
                StaticExpression::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float32)),
                },
            ..
        },
        StaticArgument::Evaluated {
            value:
                StaticExpression::ScalarLiteral {
                    value: ScalarLiteral::Integer(lane_count),
                },
            ..
        },
    ] = arguments.as_slice()
    else {
        panic!("expected evaluated vector arguments, got {arguments:?}");
    };

    assert_eq!(*lane_count, 16, "expected lane width to resolve to 16");
}

/// Materialize extension-owned associated comptime counts through projected aliases.
#[test]
fn test_extension_associated_alias_projection_materializes_comptime_counts() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
struct ImageBatch<T> {}

interface Tiled<T> {
comptime const TileRows: int;
comptime const TileCols: int;
type Tile = T[this.TileRows][this.TileCols];
}

extension<T> for ImageBatch<T> implements Tiled<T> {
comptime const TileRows: int = 8;
comptime const TileCols: int = 8;
}

declare const tile: ImageBatch<float32>.Tile;
"#,
    );
    let view = test.declare_view(module_id);
    let types = view.types();
    let tile_type_id = view.expect_namespace_value_type_id("tile");

    let Type::ArraySized {
        element: outer_element,
        count: outer_count,
        ..
    } = types.get_type(tile_type_id)
    else {
        panic!(
            "expected outer fixed-size tile projection, got {:?}",
            types.get_type(tile_type_id)
        );
    };
    let Type::ArraySized {
        count: inner_count, ..
    } = types.get_type(*outer_element)
    else {
        panic!(
            "expected inner fixed-size tile projection, got {:?}",
            types.get_type(*outer_element)
        );
    };

    let outer_literal = test
        .compiler
        .integer_literal_value_for_type_id(*outer_count, types);
    let inner_literal = test
        .compiler
        .integer_literal_value_for_type_id(*inner_count, types);

    assert_eq!(
        outer_literal,
        Some(8),
        "expected outer tile count to resolve to 8: outer_count={:?}",
        types.get_type(*outer_count),
    );
    assert_eq!(
        inner_literal,
        Some(8),
        "expected inner tile count to resolve to 8: inner_count={:?}",
        types.get_type(*inner_count),
    );
}

/// Preserve owner substitutions for associated comptime value projections.
#[test]
fn test_associated_comptime_projection_preserves_owner_substitutions() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_declare_module_with_source(
        "test.ds",
        r#"
class SegmentPlan<Row> {
comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}

declare const logSegment: SegmentPlan<string>.SegmentBytes;
declare const metricSegment: SegmentPlan<int32>.SegmentBytes;
"#,
    );

    let module = test.program.module_descriptor(module_id);
    let module = module.as_ref();
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir_data(module_id, profile);
    let tree = dir.tree.clone();
    let symbols = dir.symbols.clone();
    let mut types = dir.types.clone();
    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);

    let log_symbol = symbols
        .find_active_symbol_up_to(
            namespace_scope,
            StaticKey::Name(test.program.strings.intern("logSegment")),
            LocalScopeMark::end(),
        )
        .map(|symbol: LocalSymbolId| symbol.into_global(module.id))
        .expect("expected logSegment symbol");
    let metric_symbol = symbols
        .find_active_symbol_up_to(
            namespace_scope,
            StaticKey::Name(test.program.strings.intern("metricSegment")),
            LocalScopeMark::end(),
        )
        .map(|symbol: LocalSymbolId| symbol.into_global(module.id))
        .expect("expected metricSegment symbol");
    let log_ty_id = types
        .get_value_type_id(log_symbol)
        .expect("expected logSegment type");
    let metric_ty_id = types
        .get_value_type_id(metric_symbol)
        .expect("expected metricSegment type");

    let segment_plan_symbol = symbols
        .find_active_symbol_up_to(
            namespace_scope,
            StaticKey::Name(test.program.strings.intern("SegmentPlan")),
            LocalScopeMark::end(),
        )
        .map(|symbol: LocalSymbolId| symbol.into_global(module.id))
        .expect("expected SegmentPlan symbol");
    let log_pattern_id = symbols
        .get_symbol(log_symbol.local_id)
        .primary_declaration
        .expect("expected primary declaration for logSegment")
        .local_id
        .try_into_typed::<Pattern>()
        .expect("expected pattern declaration for logSegment");
    let log_declarator_id = tree
        .get_parent(log_pattern_id.id)
        .expect("expected parent declarator for logSegment pattern")
        .into_typed::<Declarator>();
    let log_declarator = tree.get(log_declarator_id);
    let log_member_expression_id = log_declarator
        .ty
        .expect("expected type annotation for logSegment");
    let Expression::Member { left, name, .. } = tree.get(log_member_expression_id) else {
        panic!("expected member type annotation for logSegment");
    };
    let options = test.analyze_context_options_for_module(module.id);
    let compiler_context = test.context();
    let mut type_context = TypeContext::new(
        &compiler_context,
        module,
        profile,
        &options,
        &tree,
        &symbols,
        &mut types,
        AnalyzeIndex::default(),
    );
    let member_selection = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.resolve_type_member_symbol(
                &mut type_context,
                log_member_expression_id,
                *left,
                StaticKey::Name(name.expect("expected member name")),
                true,
                true,
            )
        },
    );
    let member_selection = member_selection
        .expect("expected member selection query to succeed")
        .expect("expected member selection");
    let receiver_argument_count = match member_selection {
        TypeMemberResolution::Associated(selection) => selection.receiver_arguments.len(),
        _ => 0,
    };
    assert_eq!(
        receiver_argument_count, 1,
        "expected associated selection to preserve one receiver argument"
    );
    let segment_bytes_symbol = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<Option<_>, AnalyzeError>(compiler.query_static_member_symbol(
                _context.revision(),
                module,
                profile,
                segment_plan_symbol,
                StaticKey::Name(test.program.strings.intern("SegmentBytes")),
                &tree,
                &symbols,
            ))
        },
    )
    .expect("failed to query SegmentBytes symbol");
    let segment_bytes_symbol = segment_bytes_symbol.expect("expected SegmentBytes symbol");

    let string_type_id = types.insert_type_from_any(
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
        dir.roots[0].into_any(),
    );
    let receiver_arguments = vec![StaticArgument::Evaluated {
        name: None,
        value: StaticExpression::Type { ty: string_type_id },
    }];
    let options = test.analyze_context_options_for_module(module.id);
    let compiler_context = test.context();
    let mut ctx = TypeContext::new(
        &compiler_context,
        module,
        profile,
        &options,
        &tree,
        &symbols,
        &mut types,
        AnalyzeIndex::default(),
    );
    let substitutions = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<std::collections::HashMap<_, _>, AnalyzeError>(
                compiler.build_type_parameter_substitutions_for_symbol(
                    &mut ctx,
                    segment_plan_symbol,
                    dir.roots[0].into_any(),
                    &receiver_arguments,
                ),
            )
        },
    )
    .expect("failed to build type parameter substitutions");
    let mut visited = std::collections::HashSet::new();
    let direct_projection = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.resolve_static_constant_reference_for_mode(
                &mut ctx.reborrow(),
                segment_bytes_symbol,
                Some(&substitutions),
                &mut visited,
                StaticConstantResolutionMode::InstantiatedInfer,
            )
        },
    );
    let direct_projection = direct_projection.expect("expected static projection value");
    assert!(
        matches!(
            direct_projection,
            Some(StaticExpression::ScalarLiteral {
                value: ScalarLiteral::Integer(4096)
            })
        ),
        "expected direct static projection value 4096, got {direct_projection:?}"
    );
    let log_declared_type_id = types
        .get_declared_type_id(log_declarator_id.into_global_any(module.id))
        .expect("expected declared type id for logSegment");
    let log_declared_type = types.get_type(log_declared_type_id);
    assert!(
        matches!(
            log_declared_type,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(4096))
            }
        ),
        "expected declared logSegment type 4096, got {log_declared_type:?}"
    );

    let log_ty = types.get_type(log_ty_id);
    let metric_ty = types.get_type(metric_ty_id);
    assert!(
        matches!(
            log_ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(4096))
            }
        ),
        "expected logSegment type to be 4096, got {log_ty:?}"
    );
    assert!(
        matches!(
            metric_ty,
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1024))
            }
        ),
        "expected metricSegment type to be 1024, got {metric_ty:?}"
    );
}
