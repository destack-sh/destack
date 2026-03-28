use destack_runtime::host::abi::describe::{HostAbiFunction, HostAbiModule};

/// Return one PascalCase module segment for one snake-case name.
pub(super) fn android_pascal_case(name: &str) -> String {
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

/// Return the Kotlin package segment for one module.
pub(super) fn android_package_segment(module: &HostAbiModule) -> String {
    android_pascal_case(module.name)
}

/// Return the single ingress function for one module.
pub(super) fn single_ingress(module: &HostAbiModule) -> &HostAbiFunction {
    module
        .ingress
        .first()
        .unwrap_or_else(|| panic!("missing ingress function for {}", module.name))
}
