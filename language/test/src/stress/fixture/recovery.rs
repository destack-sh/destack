use std::fmt::Write;

use super::StressMode;

/// Generate damaged import and export forms that should recover at later declarations.
pub(super) fn damaged_dependency_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 256);

    for index in 0..scale {
        match index % 4 {
            0 => {
                let _ = writeln!(
                    source,
                    "import brokenDefault{index}, {{ alpha{index} as , beta{index} }} from ;"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "export {{ missing{index} as }} from \"./module{index}.ds\";"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "import data{index} from \"./data{index}.json\" with {{ type: ;"
                );
            }
            _ => {
                let _ = writeln!(source, "export * as from \"./module{index}.ds\";");
            }
        }

        let _ = writeln!(source, "export type RecoveredDependency{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate damaged dependency item clauses with a clean following declaration.
pub(super) fn damaged_dependency_item_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 160);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "import brokenDefault{index}, {{ alpha{index} as , beta{index} }} from \"./module{index}.ds\";"
        );

        let _ = writeln!(
            source,
            "export type RecoveredDependencyItem{index} = string;"
        );
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate damaged dependency target clauses with a clean following declaration.
pub(super) fn damaged_dependency_target_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 144);

    for index in 0..scale {
        let _ = writeln!(source, "import data{index} from ;");
        let _ = writeln!(source, "export {{ missing{index} as }} from ;");

        let _ = writeln!(
            source,
            "export type RecoveredDependencyTarget{index} = string;"
        );
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate import clauses with missing targets and clean following declarations.
pub(super) fn damaged_import_target_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 96);

    for index in 0..scale {
        let _ = writeln!(source, "import data{index} from ;");
        let _ = writeln!(source, "export type RecoveredImportTarget{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate export clauses with missing targets and clean following declarations.
pub(super) fn damaged_export_target_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 112);

    for index in 0..scale {
        let _ = writeln!(source, "export {{ missing{index} as }} from ;");
        let _ = writeln!(source, "export type RecoveredExportTarget{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate export clauses with only missing targets and clean following declarations.
pub(super) fn damaged_export_target_only_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 112);

    for index in 0..scale {
        let _ = writeln!(source, "export {{ missing{index} }} from ;");
        let _ = writeln!(
            source,
            "export type RecoveredExportTargetOnly{index} = string;"
        );
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate damaged dependency attribute clauses with a clean following declaration.
pub(super) fn damaged_dependency_attribute_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 160);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "import data{index} from \"./data{index}.json\" with {{ type: ;"
        );

        let _ = writeln!(
            source,
            "export type RecoveredDependencyAttribute{index} = string;"
        );
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate damaged namespace export clauses with a clean following declaration.
pub(super) fn damaged_dependency_namespace_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 144);

    for index in 0..scale {
        let _ = writeln!(source, "export * as from \"./module{index}.ds\";");

        let _ = writeln!(
            source,
            "export type RecoveredDependencyNamespace{index} = string;"
        );
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}

/// Generate damaged binding patterns that should recover at later statements.
pub(super) fn damaged_pattern_boundaries(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 256);

    for index in 0..scale {
        match index % 5 {
            0 => {
                let _ = writeln!(
                    source,
                    "const {{ key{index}: , tail{index} }} = source{index};"
                );
            }
            1 => {
                let _ = writeln!(source, "const [head{index}, ...] = items{index};");
            }
            2 => {
                let _ = writeln!(
                    source,
                    "for (const {{ value{index}: }} of items{index}) {{ use(value{index}); }}"
                );
            }
            3 => {
                let _ = writeln!(
                    source,
                    "function brokenPattern{index}({{ value{index}: }}: Source{index}) {{}}"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const {{ nested{index}: {{ item{index}: ; }} }} = source{index};"
                );
            }
        }

        let _ = writeln!(
            source,
            "const recoveredPattern{index} = source{index}.value;"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged class and interface members that should recover at later members.
pub(super) fn damaged_member_boundaries(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 256);
    source.push_str("export interface DamagedMemberBoundary {\n");

    for index in 0..scale {
        match index % 4 {
            0 => {
                let _ = writeln!(source, "    readonly brokenField{index}: ;");
            }
            1 => {
                let _ = writeln!(source, "    method{index}(value: ;");
            }
            2 => {
                let _ = writeln!(source, "    get accessor{index}(): ;");
            }
            _ => {
                let _ = writeln!(source, "    [computed{index}: ;");
            }
        }

        let _ = writeln!(source, "    recoveredMember{index}: string;");
    }

    source.push_str("}\n\nexport interface stressRecovered {}\n");

    source
}

/// Generate damaged template expressions that should recover at later statements.
pub(super) fn damaged_template_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 256);

    for index in 0..scale {
        match index % 4 {
            0 => {
                let _ = writeln!(
                    source,
                    "const brokenTemplate{index} = tag{index}`head ${{ value{index} + ; }} tail`;"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "const brokenNestedTemplate{index} = `head ${{ call{index}(, value{index}) }} tail`;"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "const brokenTaggedTemplate{index} = tag{index}<T{index}, >`value ${{ item{index} }}`;"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "const brokenTemplateObject{index} = `head ${{ {{ key: ; }} }} tail`;"
                );
            }
        }

        let _ = writeln!(
            source,
            "const recoveredTemplate{index} = tag{index}`ok ${{value{index}}}`;"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged statement expression slots with clean following statements.
pub(super) fn damaged_statement_slot_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 176);

    for index in 0..scale {
        let _ = writeln!(source, "const brokenBeforeIf{index} = ;");
        let _ = writeln!(source, "if (flag{index}) {{ use(value{index}); }}");
        let _ = writeln!(source, "return ;");
        let _ = writeln!(source, "const recoveredAfterReturn{index} = value{index};");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged statement call heads with clean following loops.
pub(super) fn damaged_statement_call_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 160);

    for index in 0..scale {
        let _ = writeln!(source, "const brokenBeforeFor{index} = call{index}(;");
        let _ = writeln!(
            source,
            "for (const item{index} of items{index}) {{ use(item{index}); }}"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged statement object heads with clean following try statements.
pub(super) fn damaged_statement_object_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 176);

    for index in 0..scale {
        let _ = writeln!(source, "const brokenBeforeTry{index} = {{ key: ;");
        let _ = writeln!(
            source,
            "try {{ work{index}(); }} catch (error) {{ recover{index}(error); }}"
        );
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged statements immediately before likely recovery boundaries.
pub(super) fn damaged_statement_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 288);

    for index in 0..scale {
        match index % 6 {
            0 => {
                let _ = writeln!(source, "const brokenBeforeIf{index} = ;");
                let _ = writeln!(source, "if (flag{index}) {{ use(value{index}); }}");
            }
            1 => {
                let _ = writeln!(source, "const brokenBeforeFor{index} = call{index}(;");
                let _ = writeln!(
                    source,
                    "for (const item{index} of items{index}) {{ use(item{index}); }}"
                );
            }
            2 => {
                let _ = writeln!(source, "const brokenBeforeTry{index} = {{ key: ;");
                let _ = writeln!(
                    source,
                    "try {{ work{index}(); }} catch (error) {{ recover{index}(error); }}"
                );
            }
            3 => {
                let _ = writeln!(source, "return ;");
                let _ = writeln!(source, "const recoveredAfterReturn{index} = value{index};");
            }
            4 => {
                let _ = writeln!(source, "using resource{index} = ;");
                let _ = writeln!(source, "const recoveredAfterUsing{index} = value{index};");
            }
            _ => {
                let _ = writeln!(source, "throw ;");
                let _ = writeln!(source, "const recoveredAfterThrow{index} = value{index};");
            }
        }
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate alternating delimiter damage that should recover at later roots.
pub(super) fn damaged_delimiter_storms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 192);

    for index in 0..scale {
        match index % 4 {
            0 => {
                let _ = writeln!(source, ")]}} const storm{index} = value{index};");
            }
            1 => {
                let _ = writeln!(source, "const brokenStorm{index} = ((({{{{[[value{index};");
            }
            2 => {
                let _ = writeln!(
                    source,
                    "const brokenObjectStorm{index} = ({{ key: value{index} ]);"
                );
            }
            _ => {
                let _ = writeln!(source, "const brokenCallStorm{index} = call{index}([{{(;");
            }
        }

        let _ = writeln!(source, "const recoveredStorm{index} = value{index};");
    }

    source.push_str("\nexport const stressRecovered = 1;\n");

    source
}

/// Generate damaged syntax near documentation comments.
pub(super) fn damaged_documentation_boundaries(
    _mode: StressMode,
    scale: usize,
    _width: usize,
) -> String {
    let mut source = String::with_capacity(scale * 288);

    for index in 0..scale {
        match index % 4 {
            0 => {
                let _ = writeln!(
                    source,
                    "/** broken function doc {index} */\nexport function brokenDocFunction{index}(value: ;) {{}}"
                );
            }
            1 => {
                let _ = writeln!(
                    source,
                    "/** broken type doc {index} */\nexport type BrokenDocType{index} = {{ item: ; }};"
                );
            }
            2 => {
                let _ = writeln!(
                    source,
                    "/** broken value doc {index} */\nexport const brokenDocValue{index} = ;"
                );
            }
            _ => {
                let _ = writeln!(
                    source,
                    "/** broken class doc {index} */\nexport class BrokenDocClass{index} {{ method(value: ;) {{}} }}"
                );
            }
        }

        let _ = writeln!(source, "export type RecoveredDoc{index} = string;");
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}
