/// Return one PascalCase module segment for one snake-case module name.
pub(super) fn apple_pascal_case(name: &str) -> String {
    let mut output = String::new();

    for segment in name.split('_') {
        let mut characters = segment.chars();
        let Some(first_character) = characters.next() else {
            continue;
        };

        output.push(first_character.to_ascii_uppercase());
        output.extend(characters);
    }

    output
}
