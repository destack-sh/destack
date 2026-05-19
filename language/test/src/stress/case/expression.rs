use super::StressMode;

/// Generate nested try/catch value expressions.
pub(super) fn nested_try(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("function nestedTry(source: Source): Payload {\n");
    source.push_str("    return try {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "        const value{index} = try {{ read{index}(source) }} catch (error) {{ recover{index}(error) }};\n"
        ));
    }

    source.push_str(
        "        combine(source)\n    } catch (error) {\n        recover(error)\n    };\n}\n",
    );

    source
}

/// Generate long expression chains.
pub(super) fn convoluted_expressions(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const convoluted = input\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    .map((item) => item.value{index} ?? fallback{index})\n    .filter((value) => value > {index})\n"
        ));
    }

    source.push_str("    .reduce((left, right) => left + right, 0);\n");

    if mode.is_destack() {
        source.push_str("const propagated = encode(input)? + decode(input)?;\n");
    }

    source
}

/// Generate damaged expression syntax with a later recovered declaration.
pub(super) fn damaged_expression(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "const brokenExpression{index} = call{index}(, value{index});\n"
        ));
    }

    source.push_str("\nconst stressRecovered = 1;\n");

    source
}
