use super::StressMode;

/// Generate nested match cases.
pub(super) fn nested_match(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("function nestedMatch(value: Result<number, Error>): number {\n");
    source.push_str("    return match (value) {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "        Ok({index}) => match (value) {{ Ok(inner) if (inner > {index}) => inner; _ => {index} }};\n"
        ));
    }

    source.push_str("        Ok(value) => value;\n        Err(_) => 0\n    };\n}\n");

    source
}

/// Generate many complex Destack patterns.
pub(super) fn convoluted_patterns(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("function convolutedPatterns(result: Result<Point, Error>): number {\n");
    source.push_str("    return match (result) {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "        Ok(Point {{ x: ^x{index}, y: *y{index} }}) if (x{index} > y{index}) => x{index};\n"
        ));
    }

    source.push_str("        Err(error) => throw error\n    };\n}\n");

    source
}

/// Generate a damaged type pattern area with a later recovered declaration.
pub(super) fn damaged_type(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!("type BrokenType{index} = {{ value: ; }};\n"));
    }

    source.push_str("\nexport type stressRecovered = string;\n");

    source
}
