use std::collections::HashMap;
use std::sync::Arc;

use tspp_core::StringPool;
use tspp_mir as mir;
use tspp_program::{
    FunctionBuilder, FunctionExport, FunctionId, FunctionTableBuilder, Object, Signature,
    SignatureId, Symbol, TypeId, object,
};

use tspp_source::{ModuleId, PackageId};

use crate::{LinkError, LinkResult};

use super::{ProgramLinker, TypeLinker};

/// Link object functions into the Program function table.
#[derive(Debug)]
pub(crate) struct FunctionLinker<'a> {
    /// Dense program id projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> FunctionLinker<'a> {
    /// Create one function linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link program function declarations.
    pub(crate) fn link(&self) -> LinkResult<FunctionTableBuilder> {
        let mut functions = Vec::new();
        let mut exports = Vec::new();

        // project object function declarations into dense Program ids
        for (module, function_id) in self.program.functions_by_id() {
            let function = self
                .program
                .object(*module)
                .function(*function_id)
                .ok_or_else(|| {
                    self.program
                        .invalid_input(format!("missing function {function_id:?}"))
                })?;
            let program_function = self.program.function_id(*module, *function_id);
            let name = function.name;
            let signature = self.program.function_signature_id(*module, *function_id);
            let mut entry = FunctionBuilder::new(name, signature);
            if let Some(environment) = function.environment {
                entry = entry.environment(self.program.type_id(*module, environment));
            }
            functions.push((Symbol::from_raw(function.symbol.raw()), entry));
            if function.linkage.is_exported() {
                exports.push(FunctionExport::new(name, program_function));
            }
        }

        Ok(FunctionTableBuilder::new()
            .signatures(self.program.signatures().iter().cloned())
            .functions(functions)
            .exports(exports))
    }
}

impl FunctionLinker<'_> {
    /// Build dense function ids from definitions and imported symbols.
    pub(crate) fn index(
        package: PackageId,
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
        strings: &StringPool,
    ) -> LinkResult<(
        HashMap<(ModuleId, mir::FunctionId), FunctionId>,
        Vec<(ModuleId, mir::FunctionId)>,
    )> {
        let mut ids = HashMap::new();
        let mut functions = Vec::new();
        let mut symbols = HashMap::new();
        let mut bindings = HashMap::new();

        // assign every definition, the copies of one shared specialization folding into the first
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                if function.linkage.is_import() {
                    continue;
                }
                let placed = symbols.get(&function.symbol).map(|(id, _, _)| *id);
                let id = FunctionId::from(functions.len() as u32);

                // fold a shared copy into its placed definition
                if function.linkage == mir::Linkage::Shared
                    && let Some(placed) = placed
                {
                    ids.insert((*module, function_id), placed);

                    continue;
                }
                // refuse a second definition of an exported symbol
                else if function.linkage.is_exported() && placed.is_some() {
                    return Err(LinkError::invalid_input(
                        package,
                        format!(
                            "function symbol {:?} has multiple definitions",
                            function.symbol
                        ),
                    ));
                }
                // place the symbol's definition
                else if function.linkage == mir::Linkage::Shared || function.linkage.is_exported()
                {
                    symbols.insert(function.symbol, (id, *module, function));
                }

                ids.insert((*module, function_id), id);
                functions.push((*module, function_id));

                // register each program-defined binding implementation once
                if let Some(binding) = &function.binding {
                    let definition = (id, *module, function);
                    if bindings.insert(binding.name, definition).is_some() {
                        let binding = strings.get(binding.name);
                        return Err(LinkError::invalid_input(
                            package,
                            format!("binding '{binding}' has multiple definitions"),
                        ));
                    }
                }
            }
        }

        // assign host bindings as external definitions
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                let Some(binding) = &function.binding else {
                    continue;
                };
                if !function.linkage.is_import() {
                    continue;
                }

                let id = match bindings.get(&binding.name).copied() {
                    Some((id, definition_module, definition)) => {
                        if !Self::signatures_match(
                            *module,
                            function,
                            definition_module,
                            definition,
                            type_ids,
                        ) {
                            let binding = strings.get(binding.name);
                            return Err(LinkError::invalid_input(
                                package,
                                format!("binding '{binding}' has conflicting declarations"),
                            ));
                        }
                        if definition.binding.as_deref() != Some(binding.as_ref()) {
                            let binding = strings.get(binding.name);
                            return Err(LinkError::invalid_input(
                                package,
                                format!("binding '{binding}' has conflicting declarations"),
                            ));
                        }

                        id
                    }
                    None => {
                        let id = FunctionId::from(functions.len() as u32);
                        bindings.insert(binding.name, (id, *module, function));
                        functions.push((*module, function_id));
                        id
                    }
                };
                ids.insert((*module, function_id), id);
            }
        }

        // resolve remaining imports against exported definitions
        for (module, object) in objects {
            for function in object.functions() {
                let function_id = function.id;
                if !function.linkage.is_import() || function.binding.is_some() {
                    continue;
                }
                let Some((id, definition_module, definition)) =
                    symbols.get(&function.symbol).copied()
                else {
                    return Err(LinkError::invalid_input(
                        package,
                        format!("function symbol {:?} is undefined", function.symbol),
                    ));
                };
                if !Self::signatures_match(
                    *module,
                    function,
                    definition_module,
                    definition,
                    type_ids,
                ) {
                    return Err(LinkError::invalid_input(
                        package,
                        format!(
                            "function symbol {:?} has conflicting signatures",
                            function.symbol
                        ),
                    ));
                }

                ids.insert((*module, function_id), id);
            }
        }

        Ok((ids, functions))
    }

    /// Return whether two functions have the same callable signature.
    pub(crate) fn signatures_match(
        left_module: ModuleId,
        left: &object::Function,
        right_module: ModuleId,
        right: &object::Function,
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> bool {
        if left.lifetimes.len() != right.lifetimes.len()
            || left.parameters.len() != right.parameters.len()
        {
            return false;
        }

        // compare parameter types
        for (left_parameter, right_parameter) in left.parameters.iter().zip(&right.parameters) {
            if !TypeLinker::same(
                left_module,
                left_parameter.ty,
                right_module,
                right_parameter.ty,
                type_ids,
            ) {
                return false;
            }
        }

        // compare result and optional closure environment types
        let result_matches = TypeLinker::same(
            left_module,
            left.result,
            right_module,
            right.result,
            type_ids,
        );
        let environment_matches = match (left.environment, right.environment) {
            (Some(left), Some(right)) => {
                TypeLinker::same(left_module, left, right_module, right, type_ids)
            }
            (None, None) => true,
            _ => false,
        };

        result_matches && environment_matches
    }

    /// Build one canonical signature table for declarations and signature types.
    pub(crate) fn signatures(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> (
        Vec<Signature>,
        HashMap<(ModuleId, mir::FunctionId), SignatureId>,
        HashMap<(ModuleId, mir::TypeId), SignatureId>,
    ) {
        let mut signatures = Vec::new();
        let mut function_signatures = HashMap::new();
        let mut type_signatures = HashMap::new();

        // assign callable declarations first in stable object order
        for (module, object) in objects {
            for function in object.functions() {
                let signature = Signature {
                    parameters: function
                        .parameters
                        .iter()
                        .map(|parameter| type_ids[&(*module, parameter.ty)])
                        .collect(),
                    result: type_ids[&(*module, function.result)],
                };
                let id = Self::insert_signature(&mut signatures, signature);
                function_signatures.insert((*module, function.id), id);
            }
        }

        // assign explicit signature types through the same canonical table
        for (module, object) in objects {
            for ty in object.types() {
                let mir::Type::FunctionSignature {
                    parameters, result, ..
                } = &ty.definition
                else {
                    continue;
                };
                let signature = Signature {
                    parameters: parameters
                        .iter()
                        .map(|parameter| type_ids[&(*module, parameter.ty)])
                        .collect(),
                    result: type_ids[&(*module, *result)],
                };
                let id = Self::insert_signature(&mut signatures, signature);
                type_signatures.insert((*module, ty.id), id);
            }
        }

        (signatures, function_signatures, type_signatures)
    }

    /// Insert one canonical signature or return its existing dense id.
    pub(crate) fn insert_signature(
        signatures: &mut Vec<Signature>,
        signature: Signature,
    ) -> SignatureId {
        let index = signatures
            .iter()
            .position(|existing| *existing == signature)
            .unwrap_or_else(|| {
                signatures.push(signature);

                signatures.len() - 1
            });

        SignatureId(index as u32)
    }
}
