use super::StressMode;

/// Generate a large comment attachment surface.
pub(super) fn large_trivia(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("/**\n * Large trivia keeps attached documentation stable.\n */\n");
    source.push_str("export function documented(value: number): number {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    // preserve comment {index}\n    const value{index} = value + /* inline {index} */ {index};\n"
        ));
    }

    source.push_str("    return value;\n}\n");

    source
}

/// Generate dense trivia around every syntax boundary.
pub(super) fn trivia_wall(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("export const triviaWall = {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    /** key {index} */\n    key{index}: /* left {index} */ value{index} /* right {index} */,\n"
        ));
    }

    source.push_str("};\n\n");
    source.push_str("export function useTriviaWall() {\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    // statement {index}\n    consume(/* argument {index} */ triviaWall.key{index});\n"
        ));
    }

    source.push_str("}\n");

    source
}

/// Generate damaged trivia with a later recovered declaration.
pub(super) fn damaged_trivia(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "/** damaged trivia {index} */\nconst brokenTrivia{index} = ;\n"
        ));
    }

    source.push_str("\nexport const stressRecovered = 2;\n");

    source
}
