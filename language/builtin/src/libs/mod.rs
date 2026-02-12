mod lib;
mod source;
mod std;

pub use lib::*;
pub use source::*;
pub use std::*;

/// Look up a builtin lib by name.
pub fn builtin_lib(name: &str) -> Option<&'static BuiltinLib> {
    if name == STD_LIB.name {
        return Some(&STD_LIB);
    }

    LIBS.iter().find(|lib| lib.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure the full profile keeps node and bun runtime libs.
    #[cfg(feature = "full")]
    #[test]
    fn test_profile_full_keeps_runtime_libs() {
        assert!(builtin_lib("node").is_some(), "expected node builtin lib");
        assert!(builtin_lib("bun").is_some(), "expected bun builtin lib");
        assert!(
            builtin_lib("undici-types").is_some(),
            "expected undici-types builtin lib"
        );
    }

    /// Ensure the core profile excludes heavy runtime libs.
    #[cfg(all(feature = "core", not(feature = "full")))]
    #[test]
    fn test_profile_core_excludes_runtime_libs() {
        assert!(
            builtin_lib("node").is_none(),
            "expected node lib to be excluded"
        );
        assert!(
            builtin_lib("bun").is_none(),
            "expected bun lib to be excluded"
        );
        assert!(
            builtin_lib("undici-types").is_none(),
            "expected undici-types lib to be excluded"
        );
        assert!(
            builtin_lib("dom").is_none(),
            "expected dom lib to be excluded"
        );
        assert!(
            builtin_lib("worker").is_none(),
            "expected worker lib to be excluded"
        );
    }

    /// Ensure latest node mode exposes only the latest versioned node lib.
    #[cfg(all(
        feature = "lib-node",
        feature = "versions-latest",
        not(feature = "legacy-node")
    ))]
    #[test]
    fn test_versions_latest_uses_latest_node_lib() {
        let node_lib = builtin_lib("node").expect("expected node builtin lib");

        let has_latest_index_source = node_lib
            .sources
            .iter()
            .any(|source| source.path == "node/v24" && source.name == "index.d.ts");

        assert!(
            has_latest_index_source,
            "expected node lib to use latest v24 source in versions-latest mode"
        );
        assert!(
            builtin_lib("node.v24").is_some(),
            "expected node.v24 builtin lib in versions-latest mode"
        );
        assert!(
            builtin_lib("node.v22").is_none(),
            "expected node.v22 builtin lib to be excluded in versions-latest mode"
        );
    }

    /// Ensure latest undici mode exposes only the latest versioned undici lib.
    #[cfg(all(
        feature = "lib-undici-types",
        feature = "versions-latest",
        not(feature = "legacy-undici-types")
    ))]
    #[test]
    fn test_versions_latest_uses_latest_undici_lib() {
        let undici_lib = builtin_lib("undici-types").expect("expected undici-types builtin lib");

        let has_latest_index_source = undici_lib
            .sources
            .iter()
            .any(|source| source.path == "undici-types/v7" && source.name == "index.d.ts");

        assert!(
            has_latest_index_source,
            "expected undici-types lib to use latest v7 source in versions-latest mode"
        );
        assert!(
            builtin_lib("undici-types.v7").is_some(),
            "expected undici-types.v7 builtin lib in versions-latest mode"
        );
        assert!(
            builtin_lib("undici-types.v6").is_none(),
            "expected undici-types.v6 builtin lib to be excluded in versions-latest mode"
        );
    }
}
