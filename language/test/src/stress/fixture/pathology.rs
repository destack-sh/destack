use std::fmt::Write;

use super::StressMode;

/// Generate a multi-megabyte declaration and expression file.
pub(super) fn massive_file(mode: StressMode, scale: usize, _width: usize) -> String {
    let item_count = scale;
    let mut source = String::with_capacity(item_count * 128);

    if mode.is_declaration() {
        source.push_str("export interface MassiveShape {\n");

        for index in 0..item_count {
            let _ = writeln!(
                source,
                "    readonly item{index}: {{ readonly id: number; readonly name: string; readonly next?: MassiveShape }};"
            );
        }

        source.push_str("}\n");

        return source;
    }

    source.push_str("const massiveSeed = 1;\n");
    source.push_str("export const massiveFile = {\n");

    for index in 0..item_count {
        let _ = writeln!(
            source,
            "    item{index}: {{ id: massiveSeed + {index}, name: `item-${{massiveSeed + {index}}}`, next: item{index} ?? null }},"
        );
    }

    source.push_str("};\n");

    source
}

/// Generate a deeply parenthesized expression.
pub(super) fn deep_parentheses(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 2 + 64);
    source.push_str("const deepParentheses = ");

    for _ in 0..scale {
        source.push('(');
    }

    source.push_str("seed");

    for _ in 0..scale {
        source.push(')');
    }

    source.push_str(";\n");

    source
}

/// Generate deeply nested statement blocks.
pub(super) fn deep_block(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 48);
    source.push_str("export function deepBlock(value: number): number {\n");

    for index in 0..scale {
        let _ = writeln!(source, "if (value > {index}) {{");
    }

    source.push_str("return value;\n");

    for _ in 0..scale {
        source.push_str("}\n");
    }

    source.push_str("return 0;\n}\n");

    source
}

/// Generate deeply nested TSX elements.
pub(super) fn deep_tree(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 32);

    if mode == StressMode::Destack {
        source.push_str("const deepTree = ");
    } else {
        source.push_str("export const deepTree = ");
    }

    for index in 0..scale {
        let _ = write!(source, "<Node{index} value={{items[{index}]}}>");
    }

    source.push_str("<Leaf />");

    for index in (0..scale).rev() {
        let _ = write!(source, "</Node{index}>");
    }

    source.push_str(";\n");

    source
}

/// Generate a very wide call expression.
pub(super) fn wide_call(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 16 + 64);
    source.push_str("const wideCall = invoke(\n");

    for index in 0..scale {
        let _ = writeln!(source, "    argument{index},");
    }

    source.push_str(");\n");

    source
}

/// Generate damaged argument lists that should recover at later statements.
pub(super) fn damaged_argument_lists(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 128);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const brokenArguments{index} = invoke{index}(value{index}, {{ item: ;"
        );
        let _ = writeln!(
            source,
            "const recoveredArguments{index} = invoke{index}(value{index});"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged type member bodies that should recover at a later root.
pub(super) fn damaged_type_member_bodies(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 96);
    source.push_str("export interface DamagedTypeMemberBodies {\n");

    for index in 0..scale {
        let _ = writeln!(source, "    brokenMember{index}: {{ readonly item: ;");
        let _ = writeln!(source, "    recoveredMember{index}: string;");
    }

    source.push_str("}\n\nexport interface stressRecovered {}\n");

    source
}

/// Generate repeated delimiter damage with a later recovered declaration.
pub(super) fn damaged_delimiters(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 64);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const broken{index} = call{index}((value{index}, {{ item: ;"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged nested blocks that should recover at a later root.
pub(super) fn damaged_nested_blocks(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 72);
    source.push_str("export function damagedNestedBlocks(value: number): number {\n");

    for index in 0..scale {
        let _ = writeln!(source, "if (value > {index}) {{");
        let _ = writeln!(source, "const brokenNestedBlock{index} = ;");
    }

    source.push_str("return value;\n");

    for _ in 0..scale {
        source.push_str("}\n");
    }

    source.push_str("}\n\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged parenthesized heads that should recover at later statements.
pub(super) fn damaged_parenthesized_heads(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 96);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const brokenHead{index} = (value{index}: {{ item: ;"
        );
        let _ = writeln!(source, "const recoveredHead{index} = value{index};");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged generic heads that should recover at later statements.
pub(super) fn damaged_generic_heads(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 112);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const brokenGeneric{index} = <T{index} extends {{ item: ;"
        );
        let _ = writeln!(source, "const recoveredGeneric{index} = value{index};");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged arrow return types that should recover at later statements.
pub(super) fn damaged_arrow_return_heads(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 120);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "const brokenReturn{index} = (value{index}): {{ item: ;"
        );
        let _ = writeln!(source, "const recoveredReturn{index} = value{index};");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged function type heads that should recover at later statements.
pub(super) fn damaged_function_type_heads(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 128);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "type BrokenFunctionType{index} = (value{index}: {{ item: ;"
        );
        let _ = writeln!(source, "type RecoveredFunctionType{index} = string;");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged infix chains that should recover at later statements.
pub(super) fn damaged_infix_chains(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 128);

    for index in 0..scale {
        let _ = writeln!(source, "const brokenInfix{index} = value{index} + ;");
        let _ = writeln!(
            source,
            "const recoveredInfix{index} = value{index} + other{index};"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate a large file dominated by trivia.
pub(super) fn trivia_flood(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 96);
    source.push_str("/** massive trivia prelude */\n");

    for index in 0..scale {
        let _ = writeln!(
            source,
            "// leading comment {index}\nconst triviaValue{index} = /* before */ value{index} /* after */;"
        );
    }

    source
}
