use std::collections::HashSet;
use {destack_dir as dir, destack_mir as mir};

use destack_core::StringId;

use crate::{LowerError, LowerResult};

use crate::lower::ModuleLowerer;

/// A key that identifies a class method slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct MethodKey {
    /// The method name.
    name: StringId,
    /// The function role for accessor discrimination.
    role: Option<dir::FunctionRole>,
    /// The signature type id for the method.
    signature: dir::LocalTypeId,
}

impl MethodKey {
    /// Create a class method key for dispatch lookups.
    pub(crate) fn new(
        name: StringId,
        role: Option<dir::FunctionRole>,
        signature: dir::LocalTypeId,
    ) -> Self {
        Self {
            name,
            role,
            signature,
        }
    }

    /// Return the method name for this key.
    pub(crate) fn name(&self) -> StringId {
        self.name
    }

    /// Return the signature type id for this key.
    pub(crate) fn signature(&self) -> dir::LocalTypeId {
        self.signature
    }
}

/// A class method candidate for vtable construction.
#[derive(Debug, Clone)]
pub(crate) struct VtableMethod {
    /// The dispatch slot for overrides.
    key: MethodKey,
    /// Whether this method is declared as an override.
    is_override: bool,
    /// The member node for diagnostics.
    member_id: dir::LocalNodeId<dir::Member>,
    /// The method symbol for this implementation.
    symbol: dir::GlobalSymbolId,
}

impl VtableMethod {
    /// Return the slot key for this method.
    pub(crate) fn key(&self) -> MethodKey {
        self.key
    }

    /// Return the member id for this slot.
    pub(crate) fn member_id(&self) -> dir::LocalNodeId<dir::Member> {
        self.member_id
    }
}

impl ModuleLowerer<'_> {
    /// Write class dispatch tables.
    pub(crate) fn emit_vtables(&mut self) -> LowerResult<()> {
        // classes with object dispatch headers
        for symbol in self.vtable_class_symbols.clone() {
            self.vtable_for_symbol(symbol)?;
        }

        Ok(())
    }

    /// Write one class dispatch table.
    fn vtable_for_symbol(&mut self, symbol: dir::GlobalSymbolId) -> LowerResult<()> {
        if self.lowered_vtables.contains(&symbol) {
            return Ok(());
        }

        // check for cycles
        if self.vtable_in_progress.contains(&symbol) {
            let anchor = self
                .declaration_ids_for_symbol(symbol)
                .first()
                .copied()
                .map(|id| id.into_global_any(self.module_id))
                .map(|id| id.into_anchored(Some(self.profile)))
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: "vtable cycle missing declaration".to_string(),
                })?;
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "cycle detected while lowering vtable".to_string(),
            }
            .into());
        }
        self.vtable_in_progress.insert(symbol);

        // resolve the declaration id for this class symbol
        let declaration_id = self.declaration_ids_for_symbol(symbol).first().copied();
        let Some(declaration_id) = declaration_id else {
            return Err(LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: format!("missing class declaration for vtable symbol {symbol:?}"),
            }
            .into());
        };
        let anchor = declaration_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        // collect class methods in lineage order
        let virtual_slots = self.virtual_method_slots_for_class(symbol)?;

        // resolve the class instance mir type
        let instance_type_id = self.types.get_instance_type_id(symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(anchor),
                message: "class missing instance type".to_string(),
            }
        })?;
        let mir_type = self.lower_type(instance_type_id, anchor)?;

        // build vtable entries with fixed prefix
        let mut entries = Vec::with_capacity(virtual_slots.len() + 2);
        entries.push(mir::VtableEntry::TypeDescriptor);
        entries.push(mir::VtableEntry::Destructor { function: None });
        for method in virtual_slots {
            let function = self.method_function_id(method.member_id, method.symbol)?;
            entries.push(mir::VtableEntry::Method { function });
        }

        // table metadata and static storage
        {
            let vtable_global = self
                .vtable_globals_by_symbol
                .get(&symbol)
                .copied()
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: format!("missing vtable global for class {symbol:?}"),
                })?;
            let initializer = vtable_initializer(&entries);
            self.builder
                .tree_mut()
                .get_mut(vtable_global.global_id)
                .initializer = Some(initializer);

            let table = mir::Vtable {
                ty: mir_type,
                global: vtable_global.global_id,
                entries,
            };
            self.builder
                .tree_mut()
                .metadata
                .dispatch
                .insert_vtable(table);
        }

        // lowered table guard
        self.record_vtable(symbol)?;
        self.vtable_in_progress.shift_remove(&symbol);

        Ok(())
    }

    /// Collect the class lineage from base to derived.
    pub(crate) fn collect_class_lineage(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> Vec<dir::GlobalSymbolId> {
        // walk the extends chain from derived to base
        let mut lineage = Vec::new();
        let mut seen = HashSet::new();
        let mut current = Some(symbol);

        while let Some(current_symbol) = current {
            // stop on cycles
            if !seen.insert(current_symbol) {
                break;
            }

            // record the current symbol
            lineage.push(current_symbol);

            // advance to the base class
            current = self
                .types
                .symbol_lineage(current_symbol)
                .and_then(|lineage| lineage.extends);
        }

        // reverse to get base first order
        lineage.reverse();
        lineage
    }

    /// Collect class methods declared on a single class.
    fn collect_virtual_methods_for_class(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<VtableMethod>> {
        // collect declaration ids for the class symbol
        let declaration_ids = self.declaration_ids_for_symbol(symbol);
        let mut methods = Vec::new();

        // scan members in declaration order
        for declaration_id in declaration_ids {
            let declaration = self.dir_tree.get(declaration_id);
            // select the class members
            let members = match declaration {
                dir::Declaration::Class(declaration) => &declaration.members,
                _ => continue,
            };

            for member_id in members {
                let member = self.dir_tree.get(*member_id);
                // skip non method members
                let dir::Member::Method {
                    key,
                    signature,
                    symbol: method_symbol,
                    ..
                } = member
                else {
                    continue;
                };

                // skip non class methods
                if !self.method_is_virtual(member, signature) {
                    continue;
                }

                // resolve the method name
                let name = self.member_dispatch_name_or_error(
                    key.as_ref(),
                    signature.role,
                    member_id.into_any(),
                )?;

                // resolve the signature type id
                let signature_type_id = self.method_signature_type_id(*member_id)?;
                let key = MethodKey::new(name, signature.role, signature_type_id);

                let method_symbol = method_symbol.into_global(self.module_id);
                methods.push(VtableMethod {
                    key,
                    is_override: signature.is_override,
                    member_id: *member_id,
                    symbol: method_symbol,
                });
            }
        }

        Ok(methods)
    }

    /// Return true when a method should participate in class dispatch.
    pub(crate) fn method_is_virtual(
        &self,
        member: &dir::Member,
        signature: &dir::FunctionSignature,
    ) -> bool {
        // reject static and private methods
        if member.is_static() || self.member_is_private(member) {
            return false;
        }

        // reject constructor and new members
        if matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor) | Some(dir::FunctionRole::New)
        ) {
            return false;
        }

        true
    }

    /// Collect class method slots for a class in vtable order.
    pub(crate) fn virtual_method_slots_for_class(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<Vec<VtableMethod>> {
        // collect class methods in lineage order
        let lineage = self.collect_class_lineage(symbol);
        let mut virtual_slots = Vec::new();

        // populate slots with override reuse
        for class_symbol in lineage {
            let class_methods = self.collect_virtual_methods_for_class(class_symbol)?;
            for method in class_methods {
                // find an existing slot with matching signature
                let slot_index = virtual_slots.iter().position(|slot: &VtableMethod| {
                    slot.key.name == method.key.name
                        && slot.key.role == method.key.role
                        && self
                            .method_signatures_equivalent(slot.key.signature, method.key.signature)
                });

                match slot_index {
                    // reuse the slot for overrides
                    Some(index) => {
                        virtual_slots[index] = method;
                    }
                    // append a new slot for fresh methods
                    None => {
                        // reject overrides without a base slot
                        if method.is_override {
                            return Err(LowerError::UnsupportedConstruct {
                                anchor: self.diagnostic_anchor(
                                    method
                                        .member_id
                                        .into_global_any(self.module_id)
                                        .into_anchored(Some(self.profile)),
                                ),
                                message: "override method has no base slot".to_string(),
                            }
                            .into());
                        }

                        virtual_slots.push(method);
                    }
                }
            }
        }

        Ok(virtual_slots)
    }

    /// Check whether a class has any class methods.
    pub(crate) fn class_has_virtual_methods(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> LowerResult<bool> {
        // report whether any slots exist
        let slots = self.virtual_method_slots_for_class(symbol)?;
        Ok(!slots.is_empty())
    }

    /// Resolve the signature type id for a method member.
    pub(crate) fn method_signature_type_id(
        &self,
        member_id: dir::LocalNodeId<dir::Member>,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve the signature type from analysis
        let node_id = member_id.into_global_any(self.module_id);
        let signature_type_id = self.signature_type_id_for_node(node_id)?;

        Ok(signature_type_id)
    }

    /// Resolve the MIR function id for a method symbol.
    pub(crate) fn method_function_id(
        &self,
        member_id: dir::LocalNodeId<dir::Member>,
        method_symbol: dir::GlobalSymbolId,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        // lookup the lowered function by symbol
        let function_id = self.function_for_symbol(method_symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "missing method function".to_string(),
            }
        })?;

        Ok(function_id)
    }

    /// Return true when a member is private.
    pub(crate) fn member_is_private(&self, member: &dir::Member) -> bool {
        // check visibility carrying members directly
        match member {
            dir::Member::AssociatedType { visibility, .. }
            | dir::Member::AssociatedConst { visibility, .. }
            | dir::Member::Field { visibility, .. }
            | dir::Member::Method { visibility, .. }
            | dir::Member::Embed { visibility, .. } => {
                *visibility == Some(dir::Visibility::Private)
            }

            dir::Member::StaticBlock { .. }
            | dir::Member::ComptimeBlock { .. }
            | dir::Member::Error { .. } => false,
        }
    }
}

/// Build the static initializer for one vtable.
fn vtable_initializer(entries: &[mir::VtableEntry]) -> mir::GlobalInitializer {
    let elements = entries
        .iter()
        .map(|entry| match entry {
            mir::VtableEntry::Method { function }
            | mir::VtableEntry::Destructor {
                function: Some(function),
            } => mir::GlobalInitializer::function_address((*function).into()),
            mir::VtableEntry::TypeDescriptor | mir::VtableEntry::Destructor { function: None } => {
                mir::GlobalInitializer::zero()
            }
        })
        .collect();

    mir::GlobalInitializer::aggregate(elements)
}
