use super::{assert_format_eq_with_options, assert_output_eq, format_tree_with_options};
use crate::{
    Constant, Function, Global, GlobalInitializer, MirFormatOptions, Parameter, Tree, Type,
    TypeAlias, Value,
};
use destack_core::StringPool;

/// Formats repeated anonymous types behind synthetic aliases when configured.
#[test]
fn test_format_with_synthetic_type_aliases() {
    assert_format_eq_with_options(
        r#"
function make(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}

function use(value0: (int32, float64, boolean)): (int32, float64, boolean) {
entry0(value0: (int32, float64, boolean)):
    return value0
}
"#,
        r#"
type Tuple0 = (int32, float64, boolean);

function make(value0: int32, value1: float64, value2: boolean): Tuple0 {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: Tuple0 = tuple Tuple0 (value0, value1, value2)
    return value3
}

function use(value0: Tuple0): Tuple0 {
entry0(value0: Tuple0):
    return value0
}
"#,
        MirFormatOptions::default()
            .with_type_aliases(true)
            .with_type_alias_min_uses(1),
    );
}

/// Formats local metadata names from MIR trees when configured.
#[test]
fn test_format_local_names() {
    let mut tree = Tree::new();
    let int32_type = tree.insert_type(Type::Int {
        width: 32,
        is_signed: true,
    });
    let boolean_type = tree.insert_type(Type::Boolean);

    let strings = StringPool::new();
    let status_name = strings.intern("pkg/core:Status");
    let default_name = strings.intern("pkg/core:Status.Default");
    let is_active_name = strings.intern("pkg/core:Status.isActive");

    tree.insert(TypeAlias {
        name: status_name,
        ty: int32_type.into(),
    });
    tree.insert(Global::constant(
        default_name,
        int32_type.into(),
        GlobalInitializer::Scalar(Constant::int32(1)),
    ));
    tree.insert(Function::import(
        is_active_name,
        vec![Parameter {
            value: Value::new(0).into(),
            ty: int32_type.into(),
        }],
        boolean_type.into(),
    ));

    let output = format_tree_with_options(
        &tree,
        &strings,
        MirFormatOptions::default().with_local_names(true),
    );

    assert_output_eq(
        r#"
type Status = int32;

readonly global Status.Default: Status = 1int32

external function Status.isActive(Status): boolean
"#
        .trim(),
        output,
    );
}

/// Skips synthetic aliases when type usage stays below the configured threshold.
#[test]
fn test_format_skips_synthetic_type_aliases_below_threshold() {
    assert_format_eq_with_options(
        r#"
function make(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}
"#,
        r#"
function make(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}
"#,
        MirFormatOptions::default()
            .with_type_aliases(true)
            .with_type_alias_min_uses(3),
    );
}

/// Preserves the first explicit item comment when synthetic aliases are inserted before it.
#[test]
fn test_format_with_synthetic_aliases_before_first_item_comment() {
    assert_format_eq_with_options(
        r#"
// builder
function make(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}
"#,
        r#"
type Tuple0 = (int32, float64, boolean);

// builder
function make(value0: int32, value1: float64, value2: boolean): Tuple0 {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: Tuple0 = tuple Tuple0 (value0, value1, value2)
    return value3
}
"#,
        MirFormatOptions::default()
            .with_type_aliases(true)
            .with_type_alias_min_uses(1),
    );
}

/// Preserves explicit item comments and final trailing comments when synthetic aliases are inserted.
#[test]
fn test_format_with_synthetic_aliases_across_multiple_items_and_eof_comment() {
    assert_format_eq_with_options(
        r#"
// builder
function make(value0: int32, value1: float64, value2: boolean): (int32, float64, boolean) {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: (int32, float64, boolean) = tuple (int32, float64, boolean) (value0, value1, value2)
    return value3
}

// consumer
function use(value0: (int32, float64, boolean)): (int32, float64, boolean) {
entry0(value0: (int32, float64, boolean)):
    return value0
}
// tail
"#,
        r#"
type Tuple0 = (int32, float64, boolean);

// builder
function make(value0: int32, value1: float64, value2: boolean): Tuple0 {
entry0(value0: int32, value1: float64, value2: boolean):
    value3: Tuple0 = tuple Tuple0 (value0, value1, value2)
    return value3
}

// consumer
function use(value0: Tuple0): Tuple0 {
entry0(value0: Tuple0):
    return value0
}
// tail
"#,
        MirFormatOptions::default()
            .with_type_aliases(true)
            .with_type_alias_min_uses(1),
    );
}
