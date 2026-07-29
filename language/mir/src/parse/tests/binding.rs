use crate::{BindingAffinity, BindingEffect, BindingProvider, BindingReplay, Function};

use super::TestParser;

/// Binding attributes materialize complete typed MIR declarations.
#[test]
fn test_parse_binding_declaration() {
    let source = r#"
@binding("host.fs.open", {
    provider: "host",
    effect: "external",
    replay: "forbidden",
    affinity: "main",
    requires: ["host.fs.open"],
    platforms: ["linux"],
    families: ["unix"],
    hosts: ["native"],
})
external function open(): void
"#;
    let (tree, strings) = TestParser::new(source).parse();
    let (_, function) = tree
        .iter_nodes::<Function>()
        .next()
        .expect("binding declaration should contain one function");
    let binding = function
        .binding
        .as_ref()
        .expect("binding function should retain its declaration");

    assert_eq!(strings.get(binding.name), "host.fs.open");
    assert_eq!(binding.provider, BindingProvider::Host);
    assert_eq!(binding.effect, BindingEffect::External);
    assert_eq!(binding.replay, BindingReplay::Forbidden);
    assert_eq!(binding.affinity, BindingAffinity::Main);
    assert_eq!(binding.requires.len(), 1);
    assert_eq!(strings.get(binding.requires[0]), "host.fs.open");
    assert_eq!(binding.platforms.len(), 1);
    assert_eq!(strings.get(binding.platforms[0]), "linux");
    assert_eq!(binding.families.len(), 1);
    assert_eq!(strings.get(binding.families[0]), "unix");
    assert_eq!(binding.hosts.len(), 1);
    assert_eq!(strings.get(binding.hosts[0]), "native");
}
