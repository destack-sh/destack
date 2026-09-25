use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tspp_dir::{ExportResolution, GlobalSymbolId, LanguageItem, StaticKey, TypeRoot};
use tspp_serde::Reflect;
use tspp_source::ModuleId;

/// Compiler-known language environment for one profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct LanguageEnvironment {
    /// Language item symbols by item id.
    pub symbol_by_item: IndexMap<LanguageItem, GlobalSymbolId>,
    /// Language items by symbol id.
    pub items_by_symbol: IndexMap<GlobalSymbolId, LanguageItem>,
}

impl LanguageEnvironment {
    /// Return the package declaring the language items.
    pub fn package(&self) -> Option<tspp_source::PackageId> {
        self.symbol_by_item
            .values()
            .next()
            .map(|symbol| symbol.module_id.package_id)
    }

    /// Return one language item symbol.
    pub fn symbol(&self, item: LanguageItem) -> Option<GlobalSymbolId> {
        self.symbol_by_item.get(&item).copied()
    }

    /// Return one language symbol item.
    pub fn item(&self, symbol: GlobalSymbolId) -> Option<LanguageItem> {
        self.items_by_symbol.get(&symbol).copied()
    }
}

/// Bound-stage aggregate over the implicit modules.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct EnvironmentBound {
    /// Compiler-known language environment.
    pub language: LanguageEnvironment,
    /// Explicit global modules in load order.
    pub globals: Vec<ModuleId>,
    /// Resolved global bindings by key across the global modules.
    pub global_resolutions_by_key: IndexMap<StaticKey, Vec<ExportResolution>>,
    /// The default tree builder selected by the profile.
    pub tree: Option<GlobalSymbolId>,
}

/// Declared-stage aggregate over the implicit modules.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct EnvironmentDeclared {
    /// Extension declarations keyed by their resolved target root.
    pub extensions_by_root: IndexMap<TypeRoot, Vec<GlobalSymbolId>>,
    /// Blanket extension declarations over open parameter targets.
    pub blanket_extensions: Vec<GlobalSymbolId>,
}

impl EnvironmentBound {
    /// Return the compiler implicit modules in stable order.
    pub fn implicit_modules(&self) -> Vec<ModuleId> {
        let configured = self.globals.iter().copied();
        let language = self
            .language
            .items_by_symbol
            .keys()
            .map(|symbol| symbol.module_id);
        let globals = self
            .global_resolutions_by_key
            .values()
            .flatten()
            .flat_map(|resolution| {
                [resolution.declaration.module(), resolution.target.module()]
                    .into_iter()
                    .flatten()
            });

        let mut modules = configured
            .chain(language)
            .chain(globals)
            .collect::<Vec<_>>();

        // return each module once in stable order
        modules.sort_unstable();
        modules.dedup();

        modules
    }
}

/// Resolved compiler-known intrinsic bindings for a profile.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct LanguageIntrinsics {
    /// Intrinsic names keyed by symbol id.
    pub names_by_symbol: IndexMap<GlobalSymbolId, String>,
    /// Intrinsic symbols keyed by name.
    pub symbols_by_name: IndexMap<String, GlobalSymbolId>,
}

impl LanguageIntrinsics {
    /// Create an empty intrinsic map.
    pub fn new() -> Self {
        Self {
            names_by_symbol: IndexMap::new(),
            symbols_by_name: IndexMap::new(),
        }
    }

    /// Resolve an intrinsic name for a symbol.
    pub fn name_for_symbol(&self, symbol: GlobalSymbolId) -> Option<&str> {
        self.names_by_symbol.get(&symbol).map(String::as_str)
    }

    /// Resolve a symbol for an intrinsic name.
    pub fn symbol_for_name(&self, name: &str) -> Option<GlobalSymbolId> {
        self.symbols_by_name.get(name).copied()
    }
}
