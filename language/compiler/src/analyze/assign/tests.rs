#![allow(clippy::too_many_arguments)]

use destack_dir::{
    Asynchrony, FloatType, FunctionCardinality, GlobalSymbolId, IntType, LocalNodeIdAny,
    LocalSymbolId, LocalTypeId, NodeTree, PrimitiveType, ScalarLiteral, StaticArgument,
    StaticExpression, StaticKey, StringId, SymbolTable, SymbolType, Type, TypeElement, TypeField,
    TypeIndexSignature, TypeLiteral, TypeTable,
};
use destack_source::{FileContent, Span};
use destack_workspace::{Module, ProfileId, Ref};

use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::{
    AnalyzeError, AnalyzeOptions, Assignability, Compiler, TestProgram, run_to_completion,
};

/// Insert a type with a shared source id.
fn insert_test_type(types: &mut TypeTable, source_id: LocalNodeIdAny, ty: Type) -> LocalTypeId {
    // keep type sources consistent in tests
    types.insert_type_from_any(ty, source_id)
}

/// Find a symbol id by name and type.
fn expect_symbol_by_name(
    symbols: &SymbolTable,
    name: StringId,
    symbol_type: SymbolType,
) -> GlobalSymbolId {
    // scan for a matching symbol
    for local_id in symbols.active_symbol_ids() {
        let symbol = symbols.get_symbol(local_id);
        if symbol.ty == symbol_type && symbol.key == Some(StaticKey::Name(name)) {
            return LocalSymbolId::new_typed(local_id.id, symbol.ty).into_global(symbols.module_id);
        }
    }

    panic!("expected symbol");
}

/// Check assignability for test types through the analyze type context.
fn is_type_assignable(
    compiler: &Compiler,
    module: &Module,
    profile: ProfileId,
    tree: &NodeTree,
    symbols: &SymbolTable,
    target_id: LocalTypeId,
    source_id: LocalTypeId,
    types: &mut TypeTable,
    options: &AnalyzeOptions,
) -> Assignability {
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());
    let revision = compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"));
    let compiler_context = compiler
        .context(revision)
        .unwrap_or_else(|error| panic!("{error}"));

    let mut ctx = TypeContext::new(
        &compiler_context,
        module,
        profile,
        options,
        tree,
        symbols,
        types,
        AnalyzeIndex::default(),
    );
    let reference = Ref::for_workspace_root(compiler.repository.workspace_root());
    let revision = compiler
        .repository
        .current(&reference)
        .unwrap_or_else(|error| panic!("missing current workspace revision: {error}"));

    run_to_completion(compiler, revision, |compiler, _context| {
        Ok::<_, AnalyzeError>(compiler.is_type_assignable(&mut ctx, target_id, source_id))
    })
    .unwrap_or_else(|error| panic!("failed to compute test assignability: {error:?}"))
}

/// Number is assignable to number.
#[test]
fn test_analyze_assignability_same_primitive() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: number = 42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id_for_root();
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            number_ty,
            number_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// ArrayBuffer is assignable to ArrayBufferLike.
#[test]
fn test_analyze_assignability_alias_index_access() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let source = r#"
interface ArrayBuffer {}

interface ArrayBufferTypes {
ArrayBuffer: ArrayBuffer
}

type ArrayBufferLike = ArrayBufferTypes[keyof ArrayBufferTypes]
"#;
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();
    test.compile_check_clean();

    // load module state
    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    // resolve symbol ids
    let array_buffer_name = test.program.strings.intern("ArrayBuffer");
    let array_buffer_like_name = test.program.strings.intern("ArrayBufferLike");
    let array_buffer_symbol =
        expect_symbol_by_name(symbols, array_buffer_name, SymbolType::Interface);
    let array_buffer_like_symbol =
        expect_symbol_by_name(symbols, array_buffer_like_name, SymbolType::TypeAlias);

    // build reference types
    let array_buffer_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: array_buffer_symbol,
            generic_arguments: None,
        },
    );
    let array_buffer_like_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: array_buffer_like_symbol,
            generic_arguments: None,
        },
    );

    // assert assignability
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            array_buffer_like_ty,
            array_buffer_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// String is not assignable to number.
#[test]
fn test_analyze_assignability_different_primitives() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            number_ty,
            string_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Literal 42 is assignable to number.
#[test]
fn test_analyze_assignability_literal_to_primitive() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let literal_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            number_ty,
            literal_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Anything is assignable to any.
#[test]
fn test_analyze_assignability_any() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let any_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Any,
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            any_ty,
            number_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Never is assignable to anything (bottom type).
#[test]
fn test_analyze_assignability_never_source() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let never_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Never,
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            number_ty,
            never_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Nothing is assignable to never (except never itself).
#[test]
fn test_analyze_assignability_never_target() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let never_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Never,
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            never_ty,
            number_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// [number, string] is assignable to [number, string].
#[test]
fn test_analyze_assignability_tuple() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let tuple_ty = insert_test_type(
        types,
        source_id,
        Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(string_ty)],
            is_readonly: false,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            tuple_ty,
            tuple_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// [number, string] is not assignable to [number].
#[test]
fn test_analyze_assignability_tuple_different_length() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let tuple_short = insert_test_type(
        types,
        source_id,
        Type::Tuple {
            elements: vec![TypeElement::new(number_ty)],
            is_readonly: false,
        },
    );
    let tuple_long = insert_test_type(
        types,
        source_id,
        Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(string_ty)],
            is_readonly: false,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            tuple_short,
            tuple_long,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// number[] is assignable to number[].
#[test]
fn test_analyze_assignability_array() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let array_ty = insert_test_type(
        types,
        source_id,
        Type::Array {
            element: Some(number_ty),
            is_readonly: false,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            array_ty,
            array_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// [number, number] is assignable to number[].
#[test]
fn test_analyze_assignability_tuple_to_array() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let tuple_ty = insert_test_type(
        types,
        source_id,
        Type::Tuple {
            elements: vec![TypeElement::new(number_ty), TypeElement::new(number_ty)],
            is_readonly: false,
        },
    );
    let array_ty = insert_test_type(
        types,
        source_id,
        Type::Array {
            element: Some(number_ty),
            is_readonly: false,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            array_ty,
            tuple_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// { a: number, b: string } is assignable to { a: number }.
#[test]
fn test_analyze_assignability_object_structural() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );

    let key_a = StaticKey::Name(strings.intern("a"));
    let key_b = StaticKey::Name(strings.intern("b"));

    let obj_small = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![TypeField {
                key: key_a,
                ty: number_ty,
                is_optional: false,
                is_readonly: false,
            }],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let obj_large = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![
                TypeField {
                    key: key_a,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
                TypeField {
                    key: key_b,
                    ty: string_ty,
                    is_optional: false,
                    is_readonly: false,
                },
            ],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    // larger object assignable to smaller (has all required fields)
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            obj_small,
            obj_large,
            types,
            &options
        ),
        Assignability::Assignable
    );

    // smaller object not assignable to larger (missing field b)
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            obj_large,
            obj_small,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Optional fields are not assignable to required fields.
#[test]
fn test_analyze_assignability_object_optional_field() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let required_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let optional_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: number_ty,
        is_optional: true,
        is_readonly: false,
    };

    let required_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![required_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let optional_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![optional_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            required_obj,
            optional_obj,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            optional_obj,
            required_obj,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Object call signatures must be assignable.
#[test]
fn test_analyze_assignability_object_call_signatures() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let signature_ty = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: vec![number_ty],
            return_type: Some(string_ty),
        },
    );

    let target_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: vec![signature_ty],
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let source_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: vec![signature_ty],
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let missing_call = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            source_obj,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            missing_call,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Functions with fewer parameters are assignable to targets with more parameters.
#[test]
fn test_analyze_assignability_function_param_count() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );

    let target_fn = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: vec![number_ty, string_ty],
            return_type: Some(number_ty),
        },
    );
    let source_fn_fewer = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: vec![number_ty],
            return_type: Some(number_ty),
        },
    );
    let source_fn_more = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters: vec![number_ty, string_ty, number_ty],
            return_type: Some(number_ty),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_fn,
            source_fn_fewer,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_fn,
            source_fn_more,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Functions with this parameters are contravariant in this.
#[test]
fn test_analyze_assignability_function_this_parameter() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    let key_a = StaticKey::Name(strings.intern("a"));
    let key_b = StaticKey::Name(strings.intern("b"));

    let this_small = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![TypeField {
                key: key_a,
                ty: number_ty,
                is_optional: false,
                is_readonly: false,
            }],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let this_large = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![
                TypeField {
                    key: key_a,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
                TypeField {
                    key: key_b,
                    ty: number_ty,
                    is_optional: false,
                    is_readonly: false,
                },
            ],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    let target_fn = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: Some(this_small),
            parameters: Vec::new(),
            return_type: None,
        },
    );
    let source_fn_wider_this = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: Some(this_large),
            parameters: Vec::new(),
            return_type: None,
        },
    );
    let source_fn_narrow_this = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: Vec::new(),
            this_parameter: Some(this_small),
            parameters: Vec::new(),
            return_type: None,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_fn,
            source_fn_wider_this,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            source_fn_wider_this,
            source_fn_narrow_this,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Object index signatures must be assignable.
#[test]
fn test_analyze_assignability_object_index_signatures() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let index_signature = TypeIndexSignature {
        name: strings.intern("k"),
        key_type: string_ty,
        value_type: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let matching_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let mismatched_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: string_ty,
        is_optional: false,
        is_readonly: false,
    };

    let target_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![index_signature.clone()],
        },
    );
    let source_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![index_signature.clone()],
        },
    );
    let compatible_fields = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![matching_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let incompatible_fields = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![mismatched_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let missing_index = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            source_obj,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            missing_index,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            compatible_fields,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            incompatible_fields,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// String index signatures satisfy number index signatures.
#[test]
fn test_analyze_assignability_object_index_signatures_string_source() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let number_index = TypeIndexSignature {
        name: strings.intern("k"),
        key_type: number_ty,
        value_type: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let string_index = TypeIndexSignature {
        name: strings.intern("k"),
        key_type: string_ty,
        value_type: number_ty,
        is_optional: false,
        is_readonly: false,
    };

    let target_number = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![number_index.clone()],
        },
    );
    let source_string = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![string_index.clone()],
        },
    );
    let target_string = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![string_index],
        },
    );
    let source_number = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![number_index],
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_number,
            source_string,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_string,
            source_number,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Numeric and string literal keys are compatible for fields.
#[test]
fn test_analyze_assignability_object_numeric_key_field_match() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let key = strings.intern("1");
    let target_field = TypeField {
        key: StaticKey::Name(key),
        ty: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let source_field = TypeField {
        key: StaticKey::Number(key),
        ty: number_ty,
        is_optional: false,
        is_readonly: false,
    };

    let target_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![target_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let source_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![source_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            source_obj,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Optional fields ignore undefined when matching index signatures.
#[test]
fn test_analyze_assignability_object_index_signature_optional_field() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;
    let strings = test.program.strings.clone();

    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let undefined_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        },
    );
    let target_index = TypeIndexSignature {
        name: strings.intern("k"),
        key_type: string_ty,
        value_type: number_ty,
        is_optional: false,
        is_readonly: false,
    };
    let optional_number_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: number_ty,
        is_optional: true,
        is_readonly: false,
    };
    let optional_undefined_field = TypeField {
        key: StaticKey::Name(strings.intern("a")),
        ty: undefined_ty,
        is_optional: true,
        is_readonly: false,
    };

    let target_obj = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: Vec::new(),
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: vec![target_index],
        },
    );
    let source_optional_number = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![optional_number_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );
    let source_optional_undefined = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![optional_undefined_field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            source_optional_number,
            types,
            &options
        ),
        Assignability::Assignable
    );
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_obj,
            source_optional_undefined,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Number is assignable to number | string.
#[test]
fn test_analyze_assignability_union_target() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );
    let string_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String),
        },
    );
    let union_ty = insert_test_type(
        types,
        source_id,
        Type::Union {
            elements: vec![number_ty, string_ty],
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            union_ty,
            number_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Number literal should be assignable to number type.
#[test]
fn test_type_check_let_compatible_types() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: number = 42;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// String literal should not be assignable to number type.
#[test]
fn test_type_check_let_incompatible_types() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", r#"let x: number = "hello";"#);
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Excess properties on object literals should error.
#[test]
fn test_type_check_excess_property_object_literal() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: { a: number } = { a: 1, b: 2 };");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA208"]);
}

/// Excess property diagnostics should anchor to the object literal span.
#[test]
fn test_type_check_excess_property_anchor() {
    let test = TestProgram::memory_sequential();
    let source = "let x: { a: number } = { a: 1, b: 2 };";
    let module_id = test.add_module("test.ds", source);
    test.analyze_module(module_id);
    test.compile();

    let diagnostics = test.diagnostics();
    let diagnostic_vec = diagnostics.iter();
    let diagnostic = diagnostic_vec
        .iter()
        .find(|diag| diag.code == "EA208")
        .unwrap_or_else(|| panic!("expected diagnostic EA208"));

    let file = test.file(module_id);
    let content = match file.content.payload() {
        FileContent::Text { content } => content,
        _ => panic!("expected text file content"),
    };
    let literal = "{ a: 1, b: 2 }";
    let start = content
        .find(literal)
        .unwrap_or_else(|| panic!("missing object literal in source"));
    let end = start + literal.len();
    let expected_span = Span::new(file.id, start as u32, end as u32);

    assert_eq!(diagnostic.file_id, file.id);
    assert!(
        diagnostic.primary_span.span.intersects(expected_span),
        "diagnostic span should intersect object literal span"
    );
}

/// Excess property diagnostics should be reported for each extra field.
#[test]
fn test_type_check_excess_property_multiple() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: { a: number } = { a: 1, b: 2, c: 3 };");
    test.analyze_module(module_id);
    test.compile();

    test.check_diagnostic_count("EA208", 2);
}

/// Excess property checks should respect union candidates.
#[test]
fn test_type_check_excess_property_union_candidate() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        "let x: { a: number } | { a: number, b: number } = { a: 1, b: 2 };",
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Index signatures allow extra object literal properties.
#[test]
fn test_type_check_excess_property_index_signature() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        "let x: { [key: string]: number } = { a: 1, b: 2 };",
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Boolean literal should not be assignable to string type.
#[test]
fn test_type_check_let_boolean_to_string_error() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: string = true;");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// `any` is disabled in strict mode by default.
#[test]
fn test_type_check_let_any_disabled_in_strict() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: any = 42;");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA804"]);
}

/// Function call with compatible argument types.
#[test]
fn test_type_check_function_call_compatible_args() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(x: number, y: number): number {
return x + y;
}
let result = add(1, 2);
"#,
    );
    test.analyze_module(module_id);
    test.compile_check_clean();
    test.compile();
    test.check_clean();
}

/// Function call with incompatible argument types.
#[test]
fn test_type_check_function_call_incompatible_args() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function greet(name: string): string {
return name;
}
let result = greet(42);
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Function with declared return type should have that type.
#[test]
fn test_type_check_function_return_type() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function getNumber(): number {
return 42;
}
let x: number = getNumber();
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Integer literal should be assignable to int type.
#[test]
fn test_type_check_let_int_compatible() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: int = 4;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Integer literal should be assignable to int32 type.
#[test]
fn test_type_check_let_int32_compatible() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: int32 = 42;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Float literal should be assignable to float type.
#[test]
fn test_type_check_let_float_compatible() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: float = 3.14;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Integer literal should be assignable to float type (implicit conversion).
#[test]
fn test_type_check_let_int_to_float_compatible() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: float = 42;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Value within int8 range should be valid.
#[test]
fn test_type_check_int8_range_valid() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: int8 = 127;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Value outside int8 range should fail.
#[test]
fn test_type_check_int8_range_overflow() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: int8 = 128;");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Value within uint8 range should be valid.
#[test]
fn test_type_check_uint8_range_valid() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: uint8 = 255;");
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Value outside uint8 range should fail.
#[test]
fn test_type_check_uint8_range_overflow() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: uint8 = 256;");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Negative value should not be assignable to unsigned type.
/// Now works because we have constant folding for unary negation.
#[test]
fn test_type_check_uint_negative_fails() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x: uint = -1;");
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Integer literal 42 is assignable to int32.
#[test]
fn test_analyze_assignability_literal_to_int() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let int_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
        },
    );
    let literal_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42)),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            int_ty,
            literal_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Float literal 3.14 is assignable to float64.
#[test]
fn test_analyze_assignability_literal_to_float() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let float_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float64)),
        },
    );
    let literal_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            #[allow(clippy::approx_constant)]
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Float(3.14f64)),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            float_ty,
            literal_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Integer literal 1000 is not assignable to int8 (range: minus 128 to 127).
#[test]
fn test_analyze_assignability_int_out_of_range() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let int8_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        },
    );
    let literal_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1000)),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            int8_ty,
            literal_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// int8 widens to int16, but not vice versa.
#[test]
fn test_analyze_numeric_widening_int() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let int8_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        },
    );
    let int16_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int16)),
        },
    );

    // widening allowed
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            int16_ty,
            int8_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
    // narrowing not allowed
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            int8_ty,
            int16_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Signed integers cannot widen to unsigned (may lose negative values).
#[test]
fn test_analyze_numeric_widening_signed_to_unsigned_not_allowed() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let int8_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int8)),
        },
    );
    let uint8_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            uint8_ty,
            int8_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}
/// Verify lineage chain for multilevel inheritance.
#[test]
fn test_analyze_lineage_chain_multilevel() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
class Animal { name: string = "" }
class Dog extends Animal { breed: string = "" }
class Labrador extends Dog { color: string = "" }
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();

    let animal_id = test.resolve_to_symbol("test.ds", "Animal").unwrap();
    let dog_id = test.resolve_to_symbol("test.ds", "Dog").unwrap();
    let labrador_id = test.resolve_to_symbol("test.ds", "Labrador").unwrap();

    let dir = test.artifact_dir(module_id, test.default_profile_id(module_id));
    let types = &dir.types;

    // labrador extends dog
    let labrador_lineage = types
        .get_lineage_for_symbol(labrador_id)
        .expect("Labrador should have lineage");
    assert_eq!(labrador_lineage.extends, Some(dog_id));

    // dog extends animal
    let dog_lineage = types
        .get_lineage_for_symbol(dog_id)
        .expect("Dog should have lineage");
    assert_eq!(dog_lineage.extends, Some(animal_id));

    // animal has no extends (or empty lineage)
    let animal_lineage = types.get_lineage_for_symbol(animal_id);
    assert!(
        animal_lineage.is_none() || animal_lineage.unwrap().extends.is_none(),
        "Animal should not extend anything"
    );
}

/// Verify implements creates lineage entries.
#[test]
fn test_analyze_lineage_created_for_implements() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Printable { print(): void }
interface Saveable { save(): void }
class Document implements Printable, Saveable {
print(): void {}
save(): void {}
}
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();

    let printable_id = test.resolve_to_symbol("test.ds", "Printable").unwrap();
    let saveable_id = test.resolve_to_symbol("test.ds", "Saveable").unwrap();
    let document_id = test.resolve_to_symbol("test.ds", "Document").unwrap();

    let dir = test.artifact_dir(module_id, test.default_profile_id(module_id));
    let types = &dir.types;
    let doc_lineage = types
        .get_lineage_for_symbol(document_id)
        .expect("Document should have lineage");
    assert!(
        doc_lineage.extends.is_none(),
        "Document should not extend anything"
    );
    assert_eq!(
        doc_lineage.implements.len(),
        2,
        "Document should implement two interfaces"
    );
    assert!(
        doc_lineage.implements.contains(&printable_id),
        "Document should implement Printable"
    );
    assert!(
        doc_lineage.implements.contains(&saveable_id),
        "Document should implement Saveable"
    );
}

/// Alias references preserve nominal interface conformance.
#[test]
fn test_analyze_assignability_nominal_interface_alias_reference() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype interface Add<T> {
add(other: T): T;
}

type AddVec2 = Add<Vec2>;

struct Vec2 {
x: int32;
y: int32;
}

extension of Vec2 implements Add<Vec2> {
add(other: Vec2): Vec2 {
return Vec2 {
x: this.x + other.x,
y: this.y + other.y,
};
}
}
"#,
    );
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let add_vec2_name = test.program.strings.intern("AddVec2");
    let vec2_name = test.program.strings.intern("Vec2");
    let add_vec2_symbol = expect_symbol_by_name(symbols, add_vec2_name, SymbolType::TypeAlias);
    let vec2_symbol = expect_symbol_by_name(symbols, vec2_name, SymbolType::Struct);
    let target_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: add_vec2_symbol,
            generic_arguments: None,
        },
    );
    let source_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: vec2_symbol,
            generic_arguments: None,
        },
    );

    // capture the prepared alias surface
    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            target_ty,
            source_ty,
            types,
            &options,
        ),
        Assignability::Assignable,
    );
}

/// Imported nominal interfaces stay nominal across module boundaries.
#[test]
fn test_analyze_assignability_imported_nominal_interface_requires_implements() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "contract.ds",
        r#"
export newtype interface Add<T> {
add(other: T): T;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Add } from "./contract.ds";

struct Vec2 {
x: int32;
y: int32;

add(other: Vec2): Vec2 {
return Vec2 {
x: this.x + other.x,
y: this.y + other.y,
};
}
}
"#,
    );
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let add_name = test.program.strings.intern("Add");
    let vec2_name = test.program.strings.intern("Vec2");
    let add_symbol = symbols
        .active_symbol_ids()
        .find_map(|local_id| {
            let symbol = symbols.get_symbol(local_id);
            (symbol.key == Some(StaticKey::Name(add_name))).then(|| {
                LocalSymbolId::new_typed(local_id.id, symbol.ty).into_global(symbols.module_id)
            })
        })
        .expect("expected imported Add symbol");
    let vec2_symbol = expect_symbol_by_name(symbols, vec2_name, SymbolType::Struct);
    let vec2_reference_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: vec2_symbol,
            generic_arguments: None,
        },
    );
    let target_ty = insert_test_type(
        types,
        source_id,
        Type::Reference {
            symbol: add_symbol,
            generic_arguments: Some(vec![StaticArgument::Evaluated {
                name: None,
                value: StaticExpression::Type {
                    ty: vec2_reference_ty,
                },
            }]),
        },
    );
    let source_ty = vec2_reference_ty;

    let assignability = is_type_assignable(
        &test.compiler,
        &module,
        profile,
        tree,
        symbols,
        target_ty,
        source_ty,
        types,
        &options,
    );
    assert_eq!(
        assignability,
        Assignability::NotAssignable,
        "expected imported nominal interface to reject structural assignment: add_symbol={add_symbol:?} add_symbol_type={:?} target={:?} source={:?}",
        add_symbol.ty(),
        types.get_type(target_ty),
        types.get_type(source_ty),
    );
}

/// Object is assignable to object.
#[test]
fn test_analyze_assignability_object_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            object_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Object literal { a: number } is assignable to object.
#[test]
fn test_analyze_assignability_object_literal_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    let object_literal_ty = insert_test_type(
        types,
        source_id,
        Type::Object {
            fields: vec![TypeField {
                key: StaticKey::Name(test.program.strings.intern("a")),
                ty: number_ty,
                is_optional: false,
                is_readonly: false,
            }],
            call_signatures: vec![],
            construct_signatures: vec![],
            index_signatures: vec![],
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            object_literal_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Array is assignable to object.
#[test]
fn test_analyze_assignability_array_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    let array_ty = insert_test_type(
        types,
        source_id,
        Type::Array {
            element: Some(number_ty),
            is_readonly: false,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            array_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Function is assignable to object.
#[test]
fn test_analyze_assignability_function_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let void_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Void,
        },
    );

    let function_ty = insert_test_type(
        types,
        source_id,
        Type::Function {
            asynchrony: Asynchrony::Sync,
            cardinality: FunctionCardinality::Scalar,
            generic_parameters: vec![],
            this_parameter: None,
            parameters: vec![],
            return_type: Some(void_ty),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            function_ty,
            types,
            &options
        ),
        Assignability::Assignable
    );
}

/// Primitive number is NOT assignable to object.
#[test]
fn test_analyze_assignability_primitive_not_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let number_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            number_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Imported nominal interface declarators reject structural tagged initializers.
#[test]
fn test_type_check_imported_nominal_interface_tagged_initializer_requires_implements() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "contract.ds",
        r#"
export newtype interface Add<T> {
add(other: T): T;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Add } from "./contract.ds";

struct Vec2 {
x: int32;
y: int32;

add(other: Vec2): Vec2 {
return Vec2 {
x: this.x + other.x,
y: this.y + other.y,
};
}
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Generic newtype constructors require explicit wrapping.
#[test]
fn test_type_check_generic_newtype_union_constructor_requires_wrapping() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "main.ds",
        r#"
struct Ok<T> {
value: T;
}

struct Err<E> {
error: E;
}

newtype Result<T, E> = Ok<T> | Err<E>;

const value: Result<int32, string> = Ok { value: 1 };
"#,
    );
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostics(&["EA101"]);
}

/// Null is NOT assignable to object.
#[test]
fn test_analyze_assignability_null_not_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let null_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Null,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            null_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}

/// Undefined is NOT assignable to object.
#[test]
fn test_analyze_assignability_undefined_not_to_object() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");
    test.analyze_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let (module, mut dir, source_id, options) = test.artifact_dir_context(module_id, profile);
    let tree = &mut dir.tree;
    let symbols = &mut dir.symbols;
    let types = &mut dir.types;

    let object_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Object,
        },
    );

    let undefined_ty = insert_test_type(
        types,
        source_id,
        Type::TypeLiteral {
            value: TypeLiteral::Undefined,
        },
    );

    assert_eq!(
        is_type_assignable(
            &test.compiler,
            &module,
            profile,
            tree,
            symbols,
            object_ty,
            undefined_ty,
            types,
            &options
        ),
        Assignability::NotAssignable
    );
}
