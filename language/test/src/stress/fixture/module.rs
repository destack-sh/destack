use std::fmt::Write;

use super::StressMode;

/// Generate module metadata, globals, and import attributes.
pub(super) fn module_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 256);

    for index in 0..scale {
        let _ = writeln!(
            source,
            "import config{index} from \"./config{index}.toml\" with {{ type: \"json\" }};"
        );
    }

    source.push_str("\n@noHeap\nmodule {\n");
    source.push_str("    const product = \"stress\";\n");
    source
        .push_str("    const labels = {\n        feature: [\"parser\", \"formatter\"],\n    };\n");

    for index in 0..scale {
        let _ = writeln!(source, "    const flag{index} = config{index}.enabled;");
    }

    source.push_str("}\n\nglobal {\n");

    for index in 0..scale {
        let _ = writeln!(source, "    const globalStress{index}: int32 = {index};");
    }

    source.push_str("}\n\n");
    source.push_str("import.meta.product satisfies \"stress\";\n");
    source.push_str("import.meta.labels.feature satisfies readonly [\"parser\", \"formatter\"];\n");

    source
}
