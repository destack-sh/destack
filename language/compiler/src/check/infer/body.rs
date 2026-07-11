use std::ops::{Deref, DerefMut};

use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Expectation, ExpectedType, PlaceUse, TaskScope, ValueUse, Widening,
};

/// Yield targets for one generator body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct GeneratorTargets {
    /// The type of values the body yields.
    pub(in crate::check) yielded: dir::GlobalTypeId,
    /// The type yield expressions resume with.
    pub(in crate::check) resumed: dir::GlobalTypeId,
}

/// One checked body position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum BodyTarget {
    /// One source node owns the body.
    Node(dir::LocalNodeIdAny),
    /// The module's top-level statements are the body.
    Module,
}

impl BodyTarget {
    /// Return the owning node when one exists.
    pub(in crate::check) fn node(self) -> Option<dir::LocalNodeIdAny> {
        match self {
            Self::Node(node) => Some(node),
            Self::Module => None,
        }
    }
}

/// When one recorded body runs during solving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum BodyPhase {
    /// Runs in source order with every regular body.
    Main,
    /// Runs after regular bodies, reading completed failure unions.
    Handler,
}

/// One checked body recorded by the binder, in source order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct BodyOwner {
    /// The solve phase running this body.
    pub(in crate::check) phase: BodyPhase,
    /// The module owning the body.
    pub(in crate::check) module: ModuleId,
    /// The checked body position.
    pub(in crate::check) body: BodyTarget,
    /// The type the body's completion value must satisfy, when checked.
    pub(in crate::check) ret: Option<ExpectedType>,
    /// The value use the completion satisfies its target as.
    pub(in crate::check) ret_use: ValueUse,
    /// The yield targets when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
    /// The symbol bound from the body's value, with its widening.
    pub(in crate::check) binds: Option<(dir::GlobalSymbolId, Widening)>,
}

/// Checking state for one function body.
pub(in crate::check) struct BodyState<'check, 'state> {
    /// The component check state.
    pub(in crate::check) check: &'check mut CheckState<'state>,
    /// The module owning the body.
    pub(in crate::check) module: ModuleId,
    /// The return target, when the body returns a value.
    pub(in crate::check) ret: Option<dir::GlobalTypeId>,
    /// The value use the completion satisfies its target as.
    pub(in crate::check) ret_use: ValueUse,
    /// The yield targets, when the body is a generator.
    pub(in crate::check) generator: Option<GeneratorTargets>,
}

impl<'state> Deref for BodyState<'_, 'state> {
    type Target = CheckState<'state>;

    fn deref(&self) -> &Self::Target {
        self.check
    }
}

impl DerefMut for BodyState<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.check
    }
}

impl<'state> CheckState<'state> {
    /// Enter a body-less checking context.
    pub(in crate::check) fn body(&mut self, module: ModuleId) -> BodyState<'_, 'state> {
        BodyState {
            check: self,
            module,
            ret: None,
            ret_use: ValueUse::Output,
            generator: None,
        }
    }
}

impl<'check, 'state> BodyState<'check, 'state> {
    /// Check one recorded body, returning whether every judgment held.
    pub(in crate::check) fn run(
        check: &'check mut CheckState<'state>,
        owner: BodyOwner,
    ) -> CompilerResult<bool> {
        // resolve the declared target before entering the body
        let ret = match owner.ret {
            Some(ExpectedType::Type(ty)) => Some(ty),
            Some(ExpectedType::Node(site)) => {
                let ty = check.require_node_type(site.node)?;
                match check.flow_type_at(site, ty)? {
                    crate::check::Answer::Ready(ty) => Some(ty),
                    crate::check::Answer::Pending(_) => None,
                }
            }
            None => None,
        };
        let mut state = BodyState {
            check,
            module: owner.module,
            ret,
            ret_use: owner.ret_use,
            generator: owner.generator,
        };
        let checked = match owner.body {
            BodyTarget::Node(body) => state.check_body(body)?,
            BodyTarget::Module => state.check_module_body()?,
        };

        // bind the produced symbol from the body's value
        if let Some((symbol, widening)) = owner.binds {
            let ty = match widening {
                Widening::Preserve => checked.ty,
                Widening::Widen | Widening::WidenWrites => state.check.widen_type(checked.ty)?,
            };
            state.check.bind_symbol_type(symbol, ty)?;
        }

        // settle the body's obligations before the next body begins
        state.check.drain(TaskScope::Inference)?;

        Ok(checked.holds)
    }

    /// Check the module's top-level statements as one body.
    fn check_module_body(&mut self) -> CompilerResult<crate::check::Checked> {
        let module = self.module;
        let roots = self.check.module(module).expanded.roots.clone();
        let mut holds = true;
        for root in roots {
            let node = root.into_global_any(module);
            if !self.check.module(module).node_flows.contains_key(&node) {
                continue;
            }
            let site = self.check.node_site(node)?;
            holds &= self.check_node(site, PlaceUse::Read, None)?.holds;
        }
        let ty = self.check.intern_type(module, dir::Type::Void)?;

        Ok(crate::check::Checked { ty, holds })
    }

    /// Check one body expression against the return target.
    fn check_body(&mut self, body: dir::LocalNodeIdAny) -> CompilerResult<crate::check::Checked> {
        let site = self.node_site(body.into_global(self.module))?;
        let expectation = self
            .ret
            .map(|ret| Expectation::assignable(ret, site.origin(), self.ret_use));

        self.check_node(site, PlaceUse::Read, expectation)
    }
}
