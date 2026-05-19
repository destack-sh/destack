use super::StressMode;

/// Generate nested TSX trees.
pub(super) fn nested_tsx(mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    if mode == StressMode::Destack {
        source.push_str("const nestedTree = ");
    } else {
        source.push_str("export const nestedTree = ");
    }

    source.push_str("<Panel title=\"stress\">\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    <Section key={{items[{index}].id}} active={{items[{index}].enabled}}>\n        {{items[{index}].children.map((child) => <Item value={{child.value}} />)}}\n    </Section>\n"
        ));
    }

    source.push_str("</Panel>;\n");

    source
}

/// Generate ambiguous TSX expression boundaries.
pub(super) fn ambiguous_tsx(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const ambiguousTree = <Container>\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    <Item value={{items[{index}] satisfies Item}}>{{items[{index}]?.label ?? \"missing\"}}</Item>\n"
        ));
    }

    source.push_str("</Container>;\n");

    source
}

/// Generate damaged TSX with a later recovered declaration.
pub(super) fn damaged_tsx(_mode: StressMode, scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "const brokenTree{index} = <Panel><Item value={{,}} /></Panel>;\n"
        ));
    }

    source.push_str("\nconst stressRecovered = <Recovered />;\n");

    source
}
