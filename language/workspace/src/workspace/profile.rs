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
    let restrictions = &options.restrictions;

    ProfileFlags {
        no_managed: !restrictions.no_managed.is_allow(),
        no_heap: !restrictions.no_heap.is_allow(),
        no_runtime: !restrictions.no_runtime.is_allow(),
        no_dynamic_dispatch: !restrictions.no_dynamic_dispatch.is_allow(),
        no_unsafe: !restrictions.no_unsafe.is_allow(),
        no_reflection: !restrictions.no_reflection.is_allow(),
        no_unwind: !restrictions.no_unwind.is_allow(),
        no_aliasing_mutable_borrows: !restrictions.no_aliasing_mutable_borrows.is_allow(),
        no_implicit_receivers: !restrictions.no_implicit_receivers.is_allow(),
        emit_checked_types: options.emit_checked_types,
    }
}
