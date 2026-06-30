use std::fmt::Write;

use super::StressMode;

/// Generate decorator and annotation surfaces.
pub(super) fn decorator_forms(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::with_capacity(scale * 640);
    source.push_str("newtype deprecated = () | (string,);\n\n");

    for index in 0..scale {
        let _ = writeln!(source, "@deprecated(\"stress {index}\")");
        let _ = writeln!(source, "@derive(Clone, Debug)");
        let _ = writeln!(source, "struct Decorated{index}<comptime Enabled: bool> {{");
        source.push_str("    @if(Enabled)\n");
        let _ = writeln!(source, "    value{index}: int32;");
        source.push_str("    @if(!Enabled)\n");
        let _ = writeln!(source, "    fallback{index}: int32;");
        source.push_str("}\n\n");

        let _ = writeln!(source, "@capture(\"borrow\")");
        let _ = writeln!(
            source,
            "const decoratedClosure{index} = () => Decorated{index}<true> {{ value{index}: {index} }};\n"
        );
    }

    source
}
