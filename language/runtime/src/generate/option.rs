use std::collections::BTreeSet;

/// CLI options for runtime binding generation.
pub(crate) struct GeneratorOptions {
    /// Optional set of platform domains to include.
    pub(crate) domains: Option<BTreeSet<String>>,
    /// Whether to refresh existing generated stubs.
    pub(crate) refresh_stubs: bool,
}

/// Parse CLI options for runtime binding generation.
pub(crate) fn parse_generator_options() -> GeneratorOptions {
    // parse domain selectors from CLI flags
    let mut domains = BTreeSet::new();
    let mut refresh_stubs = false;
    let mut arguments = std::env::args().skip(1).peekable();

    while let Some(argument) = arguments.next() {
        // parse repeated single-domain flags
        if argument == "--domain" {
            let Some(value) = arguments.next() else {
                panic!("--domain requires a value");
            };
            if !value.trim().is_empty() {
                domains.insert(value.trim().to_string());
            }
            continue;
        }
        if let Some(value) = argument.strip_prefix("--domain=") {
            if !value.trim().is_empty() {
                domains.insert(value.trim().to_string());
            }
            continue;
        }

        // parse comma-separated domain list flags
        if argument == "--domains" {
            let Some(value) = arguments.next() else {
                panic!("--domains requires a value");
            };
            for entry in value.split(',') {
                let entry = entry.trim();
                if !entry.is_empty() {
                    domains.insert(entry.to_string());
                }
            }
            continue;
        }
        if let Some(value) = argument.strip_prefix("--domains=") {
            for entry in value.split(',') {
                let entry = entry.trim();
                if !entry.is_empty() {
                    domains.insert(entry.to_string());
                }
            }
            continue;
        }

        // refresh generated stubs in-place when they still match stub patterns
        if argument == "--refresh-stubs" {
            refresh_stubs = true;
            continue;
        }

        panic!("unsupported generate-bindings option {argument}");
    }

    if domains.is_empty() {
        return GeneratorOptions {
            domains: None,
            refresh_stubs,
        };
    }

    GeneratorOptions {
        domains: Some(domains),
        refresh_stubs,
    }
}
