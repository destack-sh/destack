use std::fmt::Write;

use super::StressMode;

/// Generate a broad valid expression syntax matrix.
pub(super) fn expression_matrix(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 320);
    source.push_str("const matrixSeed = 1;\n");

    for index in 0..scale {
        match index % 14 {
            0 => {
                let _ = writeln!(
                    source,
                    "const matrixCall{index} = factory{index}<Item{index}>(alpha{index}, ...items{index});"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "const matrixObject{index} = {{ key{index}: value{index}, get label() {{ return name{index}; }}, ...spread{index} }};"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "const matrixArray{index} = [head{index}, ...tail{index}, tail{index}?.at(index{index})];"
                );
            }
            3 => {
                let _ = writeln!(
                    source,
                    "const matrixOperator{index} = left{index} && right{index} ? value{index} ?? fallback{index} : other{index};"
                );
            }
            4 => {
                let _ = writeln!(
                    source,
                    "const matrixChain{index} = root{index}.member{index}?.call{index}(arg{index})[key{index}];"
                );
            }
            5 => {
                let _ = writeln!(
                    source,
                    "const matrixLambda{index} = (value{index}: Item{index}) => value{index} satisfies Item{index};"
                );
            }
            6 if mode.is_tsx() => {
                let _ = writeln!(
                    source,
                    "const matrixTree{index} = <Panel key={{key{index}}}><Item value={{value{index}}} /></Panel>;"
                );
            }
            7 => {
                let _ = writeln!(
                    source,
                    "const matrixTemplate{index} = tag{index}`value ${{item{index}.name}} tail`;"
                );
            }
            8 => {
                let _ = writeln!(
                    source,
                    "const matrixNew{index} = new Factory{index}<Item{index}>(value{index});"
                );
            }
            9 => {
                let _ = writeln!(
                    source,
                    "const matrixSequence{index} = (prepare{index}(), read{index}(key{index}), finish{index});"
                );
            }
            10 => {
                let _ = writeln!(
                    source,
                    "const matrixAssignment{index} = cache{index}.value ??= fallback{index};"
                );
            }
            11 => {
                let _ = writeln!(
                    source,
                    "const matrixAssertion{index} = value{index} as Item{index} satisfies Item{index};"
                );
            }
            12 => {
                let _ = writeln!(
                    source,
                    "const matrixDestructure{index} = (({{ value: item{index} }}) => item{index})(source{index});"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const matrixComputed{index} = (records{index}?.[key{index}] ?? defaults{index}).value;"
                );
            }
        }
    }

    source
}

/// Generate a broad valid type syntax matrix.
pub(super) fn type_matrix(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 360);

    for index in 0..scale {
        match index % 12 {
            0 => {
                let ty = type_wrap(
                    mode,
                    &format!(
                        "{{ readonly key{index}: T; readonly nested?: MatrixType{index}<T> }}"
                    ),
                );
                let _ = writeln!(source, "type MatrixObject{index}<T> = {ty};");
            }
            1 => {
                let ty = type_wrap(mode, "readonly [head: T, ...tail: T[]]");
                let _ = writeln!(source, "type MatrixTuple{index}<T> = {ty};");
            }
            2 => {
                let ty = type_wrap(
                    mode,
                    &format!(
                        "T extends {{ readonly tag: \"case{index}\" }} ? Value{index}<T> : never"
                    ),
                );
                let _ = writeln!(source, "type MatrixConditional{index}<T> = {ty};");
            }
            3 => {
                let ty = type_wrap(mode, "{ [Key in keyof T as `key${Key}`]: T[Key] }");
                let _ = writeln!(source, "type MatrixMapped{index}<T> = {ty};");
            }
            4 => {
                let ty = type_wrap(
                    mode,
                    &format!("MatrixObject{index}<T>[keyof MatrixObject{index}<T>]"),
                );
                let _ = writeln!(source, "type MatrixIndex{index}<T> = {ty};");
            }
            5 => {
                let ty = type_wrap(
                    mode,
                    &format!("new <Value extends T>(value: Value) => MatrixObject{index}<Value>"),
                );
                let _ = writeln!(source, "type MatrixConstructor{index}<T> = {ty};");
            }
            6 => {
                let ty = type_wrap(
                    mode,
                    &format!("(value: T, ...items: T[]) => MatrixObject{index}<T>"),
                );
                let _ = writeln!(source, "type MatrixFunction{index}<T> = {ty};");
            }
            7 => {
                let ty = type_wrap(mode, "keyof T | keyof readonly T[]");
                let _ = writeln!(source, "type MatrixKeyof{index}<T> = {ty};");
            }
            8 => {
                let ty = type_wrap(mode, "T extends infer Value ? Value : never");
                let _ = writeln!(source, "type MatrixInfer{index}<T> = {ty};");
            }
            9 => {
                let ty = type_wrap(mode, "`matrix${string}` | `case${number}`");
                let _ = writeln!(source, "type MatrixTemplate{index}<T> = {ty};");
            }
            10 => {
                let ty = type_wrap(mode, "{ new(value: T): MatrixObject0<T>; prototype: T }");
                let _ = writeln!(source, "type MatrixConstructMember{index}<T> = {ty};");
            }
            _ => {
                let ty = type_wrap(mode, "readonly (T | null | undefined)[]");
                let _ = writeln!(source, "type MatrixArray{index}<T> = {ty};");
            }
        }
    }

    source
}

/// Generate a broad damaged expression recovery matrix.
pub(super) fn damaged_expression_matrix(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 220);

    for index in 0..scale {
        match index % 8 {
            0 => {
                let _ = writeln!(
                    source,
                    "const brokenCall{index} = invoke{index}(alpha{index}, , beta{index});"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "const brokenObject{index} = {{ key{index}: , tail{index}: 1 }};"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "const brokenArray{index} = [head{index}, ..., tail{index}];"
                );
            }
            3 => {
                let _ = writeln!(
                    source,
                    "const brokenTernary{index} = flag{index} ? value{index} : ;"
                );
            }
            4 => {
                let _ = writeln!(
                    source,
                    "const brokenChain{index} = root{index}.?.member{index};"
                );
            }
            5 => {
                let _ = writeln!(
                    source,
                    "const brokenGenericCall{index} = call{index}<T{index}, , U{index}>(value{index});"
                );
            }
            6 if mode.is_tsx() => {
                let _ = writeln!(
                    source,
                    "const brokenTreeExpr{index} = <Panel><Item value={{,}} /></Panel>;"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const brokenTemplate{index} = tag{index}`value ${{,}} tail`;"
                );
            }
        }

        let _ = writeln!(
            source,
            "const recoveredExpressionMatrix{index} = value{index};"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate a broad damaged type recovery matrix.
pub(super) fn damaged_type_matrix(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 220);

    for index in 0..scale {
        match index % 7 {
            0 => {
                let _ = writeln!(source, "type BrokenUnion{index} = string | ;");
            }
            1 => {
                let _ = writeln!(
                    source,
                    "type BrokenObject{index} = {{ readonly key{index}: ;"
                );
            }
            2 => {
                let _ = writeln!(source, "type BrokenTuple{index} = [head: string, ... ;");
            }
            3 => {
                let _ = writeln!(source, "type BrokenConditional{index}<T> = T extends ;");
            }
            4 => {
                let _ = writeln!(source, "type BrokenMapped{index}<T> = {{ [Key in keyof ;");
            }
            5 => {
                let _ = writeln!(source, "type BrokenFunction{index} = (value: ;");
            }
            _ => {
                let _ = writeln!(source, "type BrokenIndex{index}<T> = T[ ;");
            }
        }

        let _ = writeln!(source, "type RecoveredTypeMatrix{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate a broad damaged declaration recovery matrix.
pub(super) fn damaged_declaration_matrix(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 260);

    for index in 0..scale {
        match index % 6 {
            0 => {
                let _ = writeln!(source, "export function brokenFunction{index}(value: ;");
                source.push_str(");\n");
            }
            1 => {
                let _ = writeln!(source, "export class BrokenClass{index} {{ method(value: ;");
                source.push_str("}\n");
            }
            2 => {
                let _ = writeln!(
                    source,
                    "export interface BrokenInterface{index} {{ field: ;"
                );
                source.push_str("}\n");
            }
            3 => {
                let _ = writeln!(source, "export enum BrokenEnum{index} {{ Case = ;");
                source.push_str("}\n");
            }
            4 => {
                let _ = writeln!(
                    source,
                    "export module BrokenModule{index} {{ export const value = ;"
                );
                source.push_str("}\n");
            }
            _ => {
                let _ = writeln!(source, "export type BrokenAlias{index}<T extends ;");
                source.push_str("> = T;\n");
            }
        }

        let _ = writeln!(
            source,
            "export const recoveredDeclarationMatrix{index} = {index};"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate a broad damaged tree recovery matrix.
pub(super) fn damaged_tree_matrix(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 220);

    for index in 0..scale {
        match index % 5 {
            0 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeClose{index} = <Panel><Item /></Wrong>;"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeAttribute{index} = <Item value={{,}} />;"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeChild{index} = <Panel>{{,}}</Panel>;"
                );
            }
            3 => {
                let _ = writeln!(source, "const brokenTreeNested{index} = <A><B /></C>;");
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const brokenTreeFragment{index} = <><Item value={{,}} /></>;"
                );
            }
        }

        let _ = writeln!(
            source,
            "const recoveredTreeMatrix{index} = <Recovered value={{value{index}}} />;"
        );
    }

    source.push_str("\nconst stressRecovered = <Recovered />;\n");

    source
}

fn type_wrap(mode: StressMode, ty: &str) -> String {
    if mode.is_destack() {
        format!("({ty})")
    } else {
        ty.to_string()
    }
}
