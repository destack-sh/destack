/// Convert one snake-case name into PascalCase.
pub(super) fn pascal_case(name: impl AsRef<str>) -> String {
    let mut output = String::new();

    for segment in name.as_ref().split('_') {
        let mut characters = segment.chars();
        let Some(first_character) = characters.next() else {
            continue;
        };

        output.push(first_character.to_ascii_uppercase());
        output.extend(characters);
    }

    output
}
