use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, PlaceUse};

impl CheckState<'_> {
    /// Infer one borrow expression from its borrowed value and lifetime.
    pub(in crate::sema) fn infer_borrow_expression(
        &mut self,
        site: FlowSite,
        mutability: Option<dir::Mutability>,
        right: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // read the borrowed value and the place it names
        let node = site.node.into_typed::<dir::Expression>();
        let right_site = self.visit_site(right.into_global_any(node.module_id))?;
        let ty = self.infer_node_type(right_site, PlaceUse::Read)?;
        let value = self.expression_value(right_site, ty)?;
        let place = self.value_place(right_site.origin(), value)?;

        // derive borrow form parameters from the place and written mutability
        let lifetime = place.lifetime;
        let requested = match mutability {
            Some(mutability) => mutability.access(),
            None => dir::Access::Mutable,
        };
        let access = self.access_literal(requested)?;

        // require the requested access from the selected place
        let origin = site.origin();
        let mut is_granted = self
            .constrain_access_assignable(origin, place.access, access)?
            .holds();

        // grant exclusivity over a mutable frame binding's own cell
        if !is_granted
            && requested == dir::Access::Exclusive
            && self.is_exclusive_slot(node.module_id, right)?
        {
            is_granted = true;
        }

        // report a place that withholds the requested access
        if !is_granted {
            let granted = self.access_of(place.access)?;
            self.report_borrow_access_not_granted(origin, requested, granted, value.ty)?;
        }

        // commit the access this borrow requires from the lent place
        if is_granted && let Some(source) = value.node {
            let is_aliased = self.type_is_aliased(origin, value.ty)?;
            self.commit_required_access(source, requested, is_aliased);
        }

        // build the borrow form from the lent place's lifetime and placement
        let placement = self.shallow_resolve(place.placement)?;
        let region = self.intern_region(lifetime, placement)?;
        let form = self.intern_borrow(region, access)?;
        let borrowed = self.intern_type(dir::Type::Form(dir::FormType {
            form,
            value: value.ty,
        }))?;
        let borrowed = self.normalize(origin, borrowed)?;

        self.commit_node_type(node.into_any(), borrowed)?;

        Ok(())
    }

    /// Return whether one borrowed expression names a mutable frame binding directly.
    fn is_exclusive_slot(
        &mut self,
        module: destack_source::ModuleId,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<bool> {
        // slot exclusivity applies to bare binding references alone
        if !self.module(module).view().get(expression).is_reference() {
            return Ok(false);
        }

        // require the reference to resolve to a single binding symbol
        let Some(resolution) = self
            .name_decision(expression.into_global_any(module))
            .cloned()
        else {
            return Ok(false);
        };

        let [symbol] = resolution.symbols() else {
            return Ok(false);
        };
        if !self.symbol_kind(*symbol)?.is_binding() {
            return Ok(false);
        }

        // require a mutable binding declared outside module scope
        let bindings = self.binding_table(symbol.module_id);
        let binding = bindings.get_symbol(symbol.local_id);
        let is_static = binding.scope.id == bindings.module_scope().id;
        let is_mutable = binding.binding_mutability != Some(dir::Mutability::Immutable);

        Ok(!is_static && is_mutable)
    }

    /// Commit the binding uses one borrow of a binding value requires from it.
    pub(in crate::sema) fn commit_required_access(
        &mut self,
        node: dir::GlobalNodeIdAny,
        requested: dir::Access,
        is_aliased: bool,
    ) {
        // exclusive access is a place requirement apart from binding mutability
        if requested == dir::Access::Exclusive {
            self.commit_access_use(node, dir::BindingUse::EXCLUSIVE);
        }

        // commit mutable access to directly stored binding values
        if requested != dir::Access::Readonly && !is_aliased {
            self.commit_access_use(node, dir::BindingUse::MUTATE);
        }
    }
}
