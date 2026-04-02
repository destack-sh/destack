use crate::config::{
    CompilerOptions, DaemonOptions, FormatterOptions, LinterOptions, RuntimeOptions,
};

/// Effective normalized module options for one revision scoped module view.
#[derive(Debug, Clone, Default)]
pub struct ModuleOptions {
    /// The effective compiler options.
    pub compiler: CompilerOptions,
    /// The effective runtime options.
    pub runtime: RuntimeOptions,
    /// The effective formatter options.
    pub formatter: FormatterOptions,
    /// The effective linter options.
    pub linter: LinterOptions,
    /// The effective daemon options.
    pub daemon: DaemonOptions,
}
