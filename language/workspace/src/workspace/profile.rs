use destack_artifact::ProfileFlags;

use crate::config::{CompilerOptions, Mode};

/// The stable profile id.
pub use destack_source::ProfileId;

/// Return whether one active mode is present.
pub fn has_mode(modes: &[String], mode: Mode) -> bool {
    modes.iter().any(|active| active == mode.name)
}

/// Derive profile identity flags from compiler options.
pub fn profile_flags_for_compiler_options(options: &CompilerOptions) -> ProfileFlags {
    ProfileFlags {
        no_managed: !options.no_managed.is_allow(),
        no_heap: !options.no_heap.is_allow(),
        no_runtime: !options.no_runtime.is_allow(),
        no_internal_import: !options.no_internal_import.is_allow(),
        no_implicit_dynamic_dispatch: !options.no_implicit_dynamic_dispatch.is_allow(),
        emit_checked_types: options.emit_checked_types,
    }
}
