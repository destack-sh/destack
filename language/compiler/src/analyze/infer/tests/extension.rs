use super::*;

/// Infer an inherent extension.
#[test]
fn test_analyze_inherent_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point {
    x: number;
    y: number;
}

extension of Point {
    magnitude(): number {
        return 0;
    }
}

extension of Point {
    distance(other: Point): number {
        return 0;
    }
}

let origin = Point { x: 0, y: 0 };
let magnitude = origin.magnitude();
let distance = origin.distance(origin);
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");

    // verify extension kinds
    let extension_kinds = extension_kinds_for_target(&view, point_symbol);
    assert_eq!(extension_kinds.len(), 2);
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Inherent)
    );

    // verify extension method return types
    let magnitude_symbol = test.resolve_to_symbol("test.ds", "magnitude").unwrap();
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let magnitude_ty_id = view
        .types()
        .get_value_type_id(magnitude_symbol)
        .expect("expected magnitude type");
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");

    assert_type!(
        view.types(),
        magnitude_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Infer a local extension (on a foreign type).
#[test]
fn test_analyze_local_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
export struct Point {
x: number;
y: number;
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { Point } from "./point.ds";

// local extension on foreign type
extension of Point {
    distance(other: Point): number { return 0; }
}

let origin = Point { x: 0, y: 0 };
let distance = origin.distance(origin);
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Point } from "./point.ds";

let origin = Point { x: 0, y: 0 };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
    test.analyze_module_and_check_clean(consumer_id);

    // load typed module data
    let view = test.view(module_id);
    let consumer_view = test.view(consumer_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");
    let consumer_point_symbol = test.canonical_symbol_for_path("consumer.ds", "Point");

    // verify extension kinds in the defining module
    let extension_kinds = extension_kinds_for_target(&view, point_symbol);
    assert!(!extension_kinds.is_empty());
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Local)
    );

    // verify extension does not leak into other modules
    let consumer_kinds = extension_kinds_for_target(&consumer_view, consumer_point_symbol);
    assert!(consumer_kinds.is_empty());

    // verify extension method return type
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Infer a named extension (on a foreign type, from a foreign extension).
#[test]
fn test_analyze_named_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
export struct Point {
    x: number;
    y: number;
}
"#,
    );
    test.add_file(
        "extensions.ds",
        r#"
import { Point } from "./point.ds";

export extension PointHelpers of Point {
    distance(): number { return 0; }
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { PointHelpers } from "./extensions.ds";
import { Point } from "./point.ds";

declare function getPoint(): Point;

let origin = getPoint();
let distance = origin.distance();
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Point } from "./point.ds";

let origin = Point { x: 0, y: 0 };
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);
    test.analyze_module_and_check_clean(consumer_id);

    // load typed module data
    let view = test.view(module_id);
    let consumer_view = test.view(consumer_id);

    // resolve the canonical target symbol
    let point_symbol = test.canonical_symbol_for_path("test.ds", "Point");
    let consumer_point_symbol = test.canonical_symbol_for_path("consumer.ds", "Point");

    // verify extension kinds in the importing module
    let extension_kinds = extension_kinds_for_target(&view, point_symbol);
    assert!(!extension_kinds.is_empty());
    assert!(
        extension_kinds
            .iter()
            .all(|kind| *kind == ExtensionKind::Nominal)
    );

    // verify extension does not appear without an import
    let consumer_kinds = extension_kinds_for_target(&consumer_view, consumer_point_symbol);
    assert!(consumer_kinds.is_empty());

    // verify extension method return type
    let distance_symbol = test.resolve_to_symbol("test.ds", "distance").unwrap();
    let distance_ty_id = view
        .types()
        .get_value_type_id(distance_symbol)
        .expect("expected distance type");
    assert_type!(
        view.types(),
        distance_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}
