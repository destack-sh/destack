/// Generate nested trees.
pub(super) fn nested_tree(scale: usize, _width: usize) -> String {
    let mut source = String::new();
    source.push_str("const nestedTree = ");

    source.push_str("<Panel title=\"stress\">\n");

    for index in 0..scale {
        source.push_str(&format!(
            "    <Section key={{items[{index}].id}} active={{items[{index}].enabled}}>\n        {{items[{index}].children.map((child) => <Item value={{child.value}} />)}}\n    </Section>\n"
        ));
    }

    source.push_str("</Panel>;\n");

    source
}

/// Generate ambiguous tree expression boundaries.
pub(super) fn ambiguous_tree(scale: usize, _width: usize) -> String {
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

/// Generate a damaged tree with a later recovered declaration.
pub(super) fn damaged_tree(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "const brokenTree{index} = <Panel><Item value={{,}} /></Panel>;\n"
        ));
    }

    source.push_str("\nconst stressRecovered = <Recovered />;\n");

    source
}

/// Generate damaged tree nesting with a later recovered declaration.
pub(super) fn damaged_tree_nesting(scale: usize, _width: usize) -> String {
    let mut source = String::new();

    for index in 0..scale {
        source.push_str(&format!(
            "const brokenTreeNesting{index} = <Panel><Item value={{items[{index}]}}><Child flag={{ ;</Panel>;\n"
        ));
        source.push_str(&format!(
            "const recoveredTreeNesting{index} = <Recovered value={{items[{index}]}} />;\n"
        ));
    }

    source.push_str("\nconst stressRecovered = <Recovered />;\n");

    source
}
