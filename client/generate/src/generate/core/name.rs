/// Convert one PascalCase or snake_case name to lower camel.
pub(in crate::generate) fn lower_camel(name: &str) -> String {
    let name = to_snake(name);
    let mut output = String::new();
    let mut uppercase = false;

    for character in name.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }

    output
}

/// Convert one snake_case or PascalCase name to UpperCamelCase.
pub(in crate::generate) fn upper_camel(name: &str) -> String {
    let name = to_snake(name);
    let mut output = String::new();
    let mut uppercase = true;

    for character in name.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            output.extend(character.to_uppercase());
            uppercase = false;
        } else {
            output.push(character);
        }
    }

    output
}

/// Convert one PascalCase name to snake_case.
pub(in crate::generate) fn to_snake(name: &str) -> String {
    let mut output = String::new();

    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            output.push('_');
        }
        output.extend(character.to_lowercase());
    }

    output
}
