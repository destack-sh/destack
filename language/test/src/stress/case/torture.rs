use super::StressMode;
use super::generator::{Generator, emit};

/// Generate one very long left associative binary expression.
pub(super) fn long_binary_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 12);
    generator.emit("const longBinaryChain = seed");

    for index in 0..scale {
        let operator = match index % 6 {
            0 => "+",
            1 => "-",
            2 => "*",
            3 => "/",
            4 => "%",
            _ => "**",
        };
        emit!(generator, " {operator} value{index}");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate one very long logical expression.
pub(super) fn long_logical_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 20);
    generator.emit("const longLogicalChain = flag0");

    for index in 1..scale {
        let operator = if index % 2 == 0 { "&&" } else { "||" };
        emit!(generator, " {operator} flag{index}");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate one very long nullish coalescing expression.
pub(super) fn long_nullish_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 16);
    generator.emit("const longNullishChain = value0");

    for index in 1..scale {
        emit!(generator, " ?? value{index}");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate one very long union and intersection type expression.
pub(super) fn long_type_operator_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 18);
    generator.emit("type LongTypeOperatorChain<T> = ");

    if mode.is_destack() {
        generator.emit("(");
    }

    generator.emit("{ readonly seed: T }");

    for index in 0..scale {
        let operator = if index % 2 == 0 { "|" } else { "&" };
        emit!(
            generator,
            " {operator} {{ readonly item{index}: Item{index}<T> }}"
        );
    }

    if mode.is_destack() {
        generator.emit(")");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate one long conditional type fallback ladder.
pub(super) fn long_conditional_type_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 34);
    generator.emit("type LongConditionalTypeChain<T> = ");

    if mode.is_destack() {
        generator.emit("(");
    }

    for index in 0..scale {
        emit!(
            generator,
            "T extends {{ readonly tag: \"case{index}\" }} ? Result{index}<T> : "
        );
    }

    generator.emit("never");

    if mode.is_destack() {
        generator.emit(")");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate one very long postfix chain.
pub(super) fn long_postfix_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 24);
    generator.emit("const longPostfixChain = root");

    for index in 0..scale {
        match index % 4 {
            0 => emit!(generator, ".member{index}"),
            1 => emit!(generator, "?.optional{index}"),
            2 => emit!(generator, "[index{index}]"),
            _ => emit!(generator, ".call{index}(arg{index})"),
        }
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate deeply nested lambda expressions.
pub(super) fn nested_lambda_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 18);
    generator.emit("const nestedLambdaChain = ");

    for index in 0..scale {
        emit!(generator, "(value{index}) => ");
    }

    generator.emit("seed");

    for index in (0..scale).rev() {
        emit!(generator, " + value{index}");
    }

    generator.emit(";\n");

    generator.finish()
}

/// Generate alternating parentheses and binary operators.
pub(super) fn parenthesized_binary_chain(mode: StressMode, scale: usize, width: usize) -> String {
    let mut generator = Generator::new(mode, scale, width, scale * 16);
    generator.emit("const parenthesizedBinaryChain = ");

    for _ in 0..scale {
        generator.emit("(");
    }

    generator.emit("seed");

    for index in 0..scale {
        emit!(generator, " + value{index})");
    }

    generator.emit(";\n");

    generator.finish()
}
