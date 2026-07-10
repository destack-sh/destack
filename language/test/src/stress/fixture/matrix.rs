use std::fmt::Write;

/// Generate a broad valid expression syntax matrix.
pub(super) fn expression_matrix(scale: usize, _width: usize) -> String {
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
            6 => {
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
                    "const matrixTuple{index} = (prepare{index}(), read{index}(key{index}), finish{index});"
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
pub(super) fn type_matrix(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 360);

    for index in 0..scale {
        match index % 12 {
            0 => {
                let _ = writeln!(
                    source,
                    "type MatrixObject{index}<T> = ({{ readonly key{index}: T; readonly nested?: MatrixType{index}<T> }});"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "type MatrixTuple{index}<T> = (readonly (head: T, ...tail: T[]));"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "type MatrixConditional{index}<T> = (T extends {{ readonly tag: \"case{index}\" }} ? Value{index}<T> : never);"
                );
            }
            3 => {
                let _ = writeln!(
                    source,
                    "type MatrixMapped{index}<T> = ({{ [Key in keyof T as `key${{Key}}`]: T[Key] }});"
                );
            }
            4 => {
                let _ = writeln!(
                    source,
                    "type MatrixIndex{index}<T> = (MatrixObject{index}<T>[keyof MatrixObject{index}<T>]);"
                );
            }
            5 => {
                let _ = writeln!(
                    source,
                    "type MatrixConstructor{index}<T> = (new <Value extends T>(value: Value) => MatrixObject{index}<Value>);"
                );
            }
            6 => {
                let _ = writeln!(
                    source,
                    "type MatrixFunction{index}<T> = ((value: T, ...items: T[]) => MatrixObject{index}<T>);"
                );
            }
            7 => {
                let _ = writeln!(
                    source,
                    "type MatrixKeyof{index}<T> = (keyof T | keyof readonly T[]);"
                );
            }
            8 => {
                let _ = writeln!(
                    source,
                    "type MatrixInfer{index}<T> = (T extends infer Value ? Value : never);"
                );
            }
            9 => {
                let _ = writeln!(
                    source,
                    "type MatrixTemplate{index}<T> = (`matrix${{string}}` | `case${{number}}`);"
                );
            }
            10 => {
                let _ = writeln!(
                    source,
                    "type MatrixConstructMember{index}<T> = ({{ new(value: T): MatrixObject0<T>; prototype: T }});"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "type MatrixArray{index}<T> = (readonly (T | null | undefined)[]);"
                );
            }
        }
    }

    source
}

/// Generate a broad damaged expression recovery matrix.
pub(super) fn damaged_expression_matrix(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 320);

    for index in 0..scale {
        match index % 16 {
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
            6 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeExpr{index} = <Panel><Item value={{,}} /></Panel>;"
                );
            }
            7 => {
                let _ = writeln!(
                    source,
                    "const brokenTemplate{index} = tag{index}`value ${{,}} tail`;"
                );
            }
            8 => {
                let _ = writeln!(source, "const brokenIndex{index} = items{index}[ ;");
            }
            9 => {
                let _ = writeln!(source, "const brokenAssertion{index} = value{index} as ;");
            }
            10 => {
                let _ = writeln!(
                    source,
                    "const brokenSatisfies{index} = value{index} satisfies ;"
                );
            }
            11 => {
                let _ = writeln!(source, "const brokenNew{index} = new Factory{index}(;");
            }
            12 => {
                let _ = writeln!(source, "const brokenArrow{index} = (value{index}: ;");
            }
            13 => {
                let _ = writeln!(
                    source,
                    "const brokenOptionalCall{index} = service{index}?.(;"
                );
            }
            14 => {
                let _ = writeln!(
                    source,
                    "const brokenParenthesized{index} = (alpha{index} + ;"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const brokenNestedObject{index} = {{ outer: {{ inner: ;"
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
pub(super) fn damaged_type_matrix(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 280);

    for index in 0..scale {
        match index % 14 {
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
            6 => {
                let _ = writeln!(source, "type BrokenIndex{index}<T> = T[ ;");
            }
            7 => {
                let _ = writeln!(
                    source,
                    "export type BrokenExportObject{index} = {{ nested: {{ value: ;"
                );
            }
            8 => {
                let _ = writeln!(source, "declare type BrokenDeclare{index}<T> = T extends ;");
            }
            9 => {
                let _ = writeln!(source, "newtype BrokenNominal{index} = {{ id: ;");
            }
            10 => {
                let _ = writeln!(source, "type BrokenGeneric{index}<T> = Result<T, ;");
            }
            11 => {
                let _ = writeln!(source, "type BrokenTemplate{index}<T> = `value ${{ ;");
            }
            12 => {
                let _ = writeln!(source, "type BrokenTupleObject{index} = [head: {{ value: ;");
            }
            _ => {
                let _ = writeln!(source, "type BrokenParenthesized{index} = (readonly ;");
            }
        }

        let _ = writeln!(source, "type RecoveredTypeMatrix{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate a broad damaged declaration recovery matrix.
pub(super) fn damaged_declaration_matrix(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 360);

    for index in 0..scale {
        match index % 12 {
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
            5 => {
                let _ = writeln!(source, "export type BrokenAlias{index}<T extends ;");
                source.push_str("> = T;\n");
            }
            6 => {
                let _ = writeln!(source, "export function brokenReturn{index}(): ;");
                source.push_str("{}\n");
            }
            7 => {
                let _ = writeln!(source, "export const brokenConst{index}: ;");
            }
            8 => {
                let _ = writeln!(source, "export class BrokenExtends{index} extends ;");
                source.push_str("{}\n");
            }
            9 => {
                let _ = writeln!(
                    source,
                    "export interface BrokenExtendsInterface{index} extends ;"
                );
                source.push_str("{}\n");
            }
            10 => {
                let _ = writeln!(source, "export enum BrokenComputedEnum{index} {{ Case = ;");
                source.push_str("}\n");
            }
            _ => {
                let _ = writeln!(source, "export newtype BrokenNewtype{index} = ;");
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
pub(super) fn damaged_tree_matrix(scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 320);

    for index in 0..scale {
        match index % 11 {
            0 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeClose{index} = <Panel><Item /></Wrong></Panel>;"
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
                let _ = writeln!(source, "const brokenTreeNested{index} = <A><B /></C></A>;");
            }
            4 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeFragment{index} = <><Item value={{,}} /></>;"
                );
            }
            5 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeOpen{index} = <Panel><Item><Child /></Panel>;"
                );
            }
            6 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeAncestorClose{index} = <Panel><Item value={{value{index}}} </Panel>;"
                );
            }
            7 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeExpression{index} = <Panel>{{value{index} + ;</Panel>;"
                );
            }
            8 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeAttributeName{index} = <Item ={{value{index}}} />;"
                );
            }
            9 => {
                let _ = writeln!(
                    source,
                    "const brokenTreeAttributeValue{index} = <Item label=\"closed\" missing= />;"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const brokenTreeNestedExpression{index} = <Panel><Item value={{{{ key: ; }}}} /></Panel>;"
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
