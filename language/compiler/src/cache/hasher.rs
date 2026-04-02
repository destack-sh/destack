use std::hash::{Hash, Hasher};

use destack_resolver::{
    EnforceExtension, ResolveOptions, Restriction, TypeScriptOptionsDiscovery,
    TypeScriptOptionsReferences,
};
use rustc_hash::FxHasher;

use crate::compile::CompilerOptions;

/// Hasher for cache context and repository image data.
pub(crate) struct CacheHasher {
    /// The underlying hash state.
    hasher: FxHasher,
}

impl CacheHasher {
    /// Create a new cache hasher.
    pub(crate) fn new() -> Self {
        Self {
            hasher: FxHasher::default(),
        }
    }

    /// Hash a value into the cache hash.
    pub(crate) fn hash_value<T: Hash>(&mut self, value: &T) {
        value.hash(&mut self.hasher);
    }

    /// Hash compiler options into the cache hash.
    pub(crate) fn hash_compiler_options(&mut self, options: &CompilerOptions) {
        // hash import options
        self.hash_value(&options.follow_imports);
        self.hash_resolve_options(&options.import_resolve);

        // hash type defaults
        self.hash_value(&options.default_int_width);
        self.hash_value(&options.default_float_width);
        self.hash_value(&options.inject_prelude);
        self.hash_value(&options.load_libraries);

        // hash emit shaping options
        self.hash_value(&options.source_map);
        self.hash_value(&options.elaborate_with_ternary);
        self.hash_value(&options.elaborate_split_declarators);
        self.hash_value(&options.elaborate_explicit_return);
        self.hash_value(&options.elaborate_parenthesize_casts);
        self.hash_value(&options.retain_comptime_as_comment);
        self.hash_value(&options.retain_comptime_comment_max_length);

        // hash emit behavior
        self.hash_value(&options.emit_overwrite);
        self.hash_value(&options.emit_create_dirs);
        self.hash_value(&options.emit_dry_run);
    }

    /// Hash resolve options into the cache hash.
    pub(crate) fn hash_resolve_options(&mut self, options: &ResolveOptions) {
        // hash basic settings
        self.hash_value(&options.cwd);
        self.hash_tsconfig_discovery(&options.tsconfig);
        self.hash_alias(&options.alias);
        self.hash_value(&options.conditions);
        self.hash_value(&options.resolve_package_json_exports);
        self.hash_value(&options.resolve_package_json_imports);
        self.hash_enforce_extension(options.enforce_extension);

        // hash extension aliases
        self.hash_value(&options.extension_alias.len());
        for (key, values) in &options.extension_alias {
            self.hash_value(key);
            self.hash_value(values);
        }

        // hash extension and module resolution settings
        self.hash_value(&options.extensions);
        self.hash_value(&options.is_fully_specified);
        self.hash_alias(&options.fallback);
        self.hash_value(&options.main_files);
        self.hash_value(&options.modules);
        self.hash_value(&options.resolve_to_context);
        self.hash_value(&options.prefer_relative);
        self.hash_value(&options.prefer_absolute);
        self.hash_restrictions(&options.restrictions);
        self.hash_value(&options.roots);
        self.hash_value(&options.canonicalize_symlinks);
        self.hash_value(&options.yarn_pnp);
    }

    /// Hash alias settings into the cache hash.
    /// Hash alias entries into the cache hash.
    fn hash_alias(&mut self, alias: &destack_resolver::Alias) {
        // hash alias entries
        self.hash_value(&alias.len());
        for (key, values) in alias {
            self.hash_value(key);
            self.hash_value(values);
        }
    }

    /// Hash resolve extension enforcement into the cache hash.
    /// Hash extension enforcement into the cache hash.
    fn hash_enforce_extension(&mut self, value: EnforceExtension) {
        // map enum to a stable tag
        let tag = match value {
            EnforceExtension::Enabled => 1_u8,
            EnforceExtension::Disabled => 2_u8,
        };

        self.hash_value(&tag);
    }

    /// Hash tsconfig discovery settings into the cache hash.
    /// Hash tsconfig discovery settings into the cache hash.
    fn hash_tsconfig_discovery(&mut self, value: &Option<TypeScriptOptionsDiscovery>) {
        // hash discovery mode
        match value {
            Some(TypeScriptOptionsDiscovery::Automatic) => {
                self.hash_value(&1_u8);
            }
            Some(TypeScriptOptionsDiscovery::Manual(location)) => {
                self.hash_value(&2_u8);
                self.hash_value(&location.config_file);
                self.hash_tsconfig_references(&location.references);
            }
            None => {
                self.hash_value(&0_u8);
            }
        }
    }

    /// Hash tsconfig reference settings into the cache hash.
    /// Hash tsconfig reference settings into the cache hash.
    fn hash_tsconfig_references(&mut self, value: &TypeScriptOptionsReferences) {
        // hash reference mode
        match value {
            TypeScriptOptionsReferences::Disabled => {
                self.hash_value(&1_u8);
            }
            TypeScriptOptionsReferences::Automatic => {
                self.hash_value(&2_u8);
            }
            TypeScriptOptionsReferences::Paths(paths) => {
                self.hash_value(&3_u8);
                self.hash_value(paths);
            }
        }
    }

    /// Hash resolver restrictions into the cache hash.
    /// Hash resolver restrictions into the cache hash.
    fn hash_restrictions(&mut self, restrictions: &[Restriction]) {
        // hash restrictions in order
        self.hash_value(&restrictions.len());
        for restriction in restrictions {
            match restriction {
                Restriction::Path(path) => {
                    self.hash_value(&1_u8);
                    self.hash_value(path);
                }
                Restriction::Function(handler) => {
                    self.hash_value(&2_u8);
                    let address = std::sync::Arc::as_ptr(handler) as *const () as usize;
                    self.hash_value(&address);
                }
            }
        }
    }

    /// Finish and return the hash value.
    pub(crate) fn finish(self) -> u64 {
        self.hasher.finish()
    }
}
