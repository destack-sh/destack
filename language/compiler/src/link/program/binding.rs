use tspp_mir as mir;
use tspp_program::{
    BindingAffinity, BindingBuilder, BindingEffect, BindingId, BindingProvider, BindingReplay,
};

use crate::LinkResult;

use super::ProgramLinker;

/// Link object binding declarations into the Program binding table.
#[derive(Debug)]
pub(crate) struct BindingLinker<'a> {
    /// The canonical Program linker.
    program: &'a ProgramLinker<'a>,
}

impl<'a> BindingLinker<'a> {
    /// Create one binding linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link runtime binding declarations in function order.
    pub(crate) fn link(&self) -> LinkResult<Vec<BindingBuilder>> {
        let mut bindings = Vec::new();

        // visit canonical function declarations in dense identity order
        for (module, function_id) in self.program.functions_by_id() {
            // resolve the object declaration behind this function identity
            let declaration = self
                .program
                .object(*module)
                .function(*function_id)
                .ok_or_else(|| {
                    self.program
                        .invalid_input(format!("missing function {function_id:?}"))
                })?;

            // skip ordinary callable functions
            let Some(binding_declaration) = &declaration.binding else {
                continue;
            };

            // project the complete binding declaration into Program identity
            let name = self.program.string(binding_declaration.name);
            let id = BindingId::from_name(name);
            let function = self.program.function_id(*module, *function_id);
            let mut binding = BindingBuilder::new(
                id,
                binding_declaration.name,
                function,
                Self::effect(binding_declaration.effect),
                Self::provider(binding_declaration.provider),
                Self::replay(binding_declaration.replay),
                Self::affinity(binding_declaration.affinity),
            )
            .requires(binding_declaration.requires.iter().copied())
            .platforms(binding_declaration.platforms.iter().copied())
            .families(binding_declaration.families.iter().copied())
            .hosts(binding_declaration.hosts.iter().copied());
            if binding_declaration.is_park {
                binding = binding.park();
            }
            if declaration.is_import() {
                binding = binding.imported();
            }
            bindings.push(binding);
        }

        Ok(bindings)
    }

    /// Project one MIR binding effect.
    const fn effect(effect: mir::BindingEffect) -> BindingEffect {
        match effect {
            mir::BindingEffect::Pure => BindingEffect::Pure,
            mir::BindingEffect::Deterministic => BindingEffect::Deterministic,
            mir::BindingEffect::External => BindingEffect::External,
        }
    }

    /// Project one MIR binding provider.
    const fn provider(provider: mir::BindingProvider) -> BindingProvider {
        match provider {
            mir::BindingProvider::Host => BindingProvider::Host,
            mir::BindingProvider::Runtime => BindingProvider::Runtime,
        }
    }

    /// Project one MIR binding replay behavior.
    const fn replay(replay: mir::BindingReplay) -> BindingReplay {
        match replay {
            mir::BindingReplay::Recordable => BindingReplay::Recordable,
            mir::BindingReplay::Forbidden => BindingReplay::Forbidden,
        }
    }

    /// Project one MIR binding affinity.
    const fn affinity(affinity: mir::BindingAffinity) -> BindingAffinity {
        match affinity {
            mir::BindingAffinity::None => BindingAffinity::None,
            mir::BindingAffinity::Worker => BindingAffinity::Worker,
            mir::BindingAffinity::Main => BindingAffinity::Main,
        }
    }
}
