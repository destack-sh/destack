use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    CallableImplementation, FunctionDeclaration, GenericInstanceKey, LifetimeParameters,
    LowerState, constructor_receiver_type, nominal_receiver_storage,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// The owner, role, and static flag of one imported member callable.
struct ImportedMember {
    /// The owning nominal symbol.
    owner: dir::GlobalSymbolId,
    /// The declared member role.
    role: Option<dir::FunctionRole>,
    /// Whether the member is declared static.
    is_static: bool,
}

impl LowerState<'_> {
    /// Declare an import header for one referenced foreign callable.
    pub(in crate::lower) fn declare_imported_function(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // declare the import once under its canonical name
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(());
        }

        // read the declared type and reject open parameters
        let declared = self.symbol_type(symbol)?;
        if self.signature_has_parameters_beyond_memory(declared)? {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an imported generic callable without instance arguments".to_string(),
            }
            .into());
        }

        // read the declared lifetimes and signature
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let (signature, owner) = self.signature(declared)?;

        // lead member callables with their receiver and qualify by owner
        let member = self.imported_member(symbol)?;
        let (name, receiver) = match &member {
            Some(member) => {
                let (name, receiver) = self.imported_member_header(
                    tree,
                    symbol,
                    member,
                    signature,
                    owner,
                    &lifetime_parameters,
                )?;

                (self.qualified_name(symbol.module_id, &name)?, receiver)
            }
            None => {
                let Some(name) = self.symbol_name(symbol)? else {
                    return Err(CompilerError::Internal {
                        message: "an imported function without a name".to_string(),
                    });
                };

                (
                    self.qualified_name(symbol.module_id, self.strings.get(name))?,
                    None,
                )
            }
        };

        // lower the signature, prepending the receiver of a method import
        let (mut parameters, result) =
            self.lower_signature(tree, declared, None, &lifetime_parameters)?;
        if let Some(receiver) = receiver {
            parameters.insert(0, receiver);
        }

        // give constructors a void result
        let is_constructor = member
            .as_ref()
            .is_some_and(|member| member.role == Some(dir::FunctionRole::Constructor));
        let result = match is_constructor {
            true => tree.intern_type(mir::Type::Void),
            false => result,
        };

        // declare the header as an external function under the imported name
        let header =
            lifetime_parameters.declare(mir::FunctionHeaderBuilder::new(self.strings, &name));
        let header = header.parameters(parameters).result(result);
        let function = tree.insert(header.imported());
        self.index_language_declaration(function, symbol)?;

        // record the declaration so later call sites resolve to it
        self.functions
            .insert(key, FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Declare the imported global behind one foreign module constant.
    pub(in crate::lower) fn declare_imported_constant(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // declare the import once under its canonical name
        if self.globals.contains_key(&symbol) {
            return Ok(());
        }

        // lower the constant's declared type outside any lifetime parameters
        let pointer_bytes = self.pointer_bytes;
        let lifetimes = LifetimeParameters::default();
        let declared = self.symbol_type(symbol)?;
        let ty = self
            .type_lowerer(tree, pointer_bytes, &lifetimes)
            .lower(declared)?;

        // declare the import at its dotted name
        let name = self.constant_name(symbol)?;
        let name = self.strings.intern(&name);
        let global = tree.insert(mir::Global::import(name, ty, mir::Mutability::Immutable));
        self.index_language_declaration(global, symbol)?;
        self.globals.insert(symbol, Ok(global));

        Ok(())
    }

    /// Declare the dotted-name extern behind one binding.
    pub(in crate::lower) fn declare_binding_function(
        &mut self,
        tree: &mut mir::Tree,
        effects: &mut mir::EffectTable,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        // declare the extern once under its canonical name
        let key = GenericInstanceKey::non_generic(symbol);
        if self.functions.contains_key(&key) {
            return Ok(());
        }

        // require a binding implementation behind the callable
        let Some(CallableImplementation::Binding { binding }) =
            self.callable_implementation(symbol)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an unnamed binding callable".to_string(),
            }
            .into());
        };

        // require a concrete signature for one extern header
        let declared = self.symbol_type(symbol)?;
        if self.signature_has_parameters_beyond_memory(declared)? {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a generic binding callable".to_string(),
            }
            .into());
        }

        // lower the signature outside any instance bindings
        let lifetime_parameters = self.lifetime_parameters(declared)?;
        let (parameters, result) =
            self.lower_host_signature(tree, declared, &lifetime_parameters)?;

        // declare the header as a host binding under the dotted extern name
        let name = self.strings.get(binding.name);
        let header =
            lifetime_parameters.declare(mir::FunctionHeaderBuilder::new(self.strings, name));
        let header = header.parameters(parameters).result(result);
        let function = tree.insert(header.imported().with_binding(binding));
        self.index_language_declaration(function, symbol)?;

        // mark bindings as observing external state
        *effects.upsert_function(function) = mir::FunctionEffect::unknown();

        // record the declaration so later call sites resolve to it
        self.functions
            .insert(key, FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Import one foreign drop hook at its instantiated instance identity.
    pub(in crate::lower) fn import_drop_hook(
        &mut self,
        tree: &mut mir::Tree,
        key: &GenericInstanceKey,
        storage: mir::LocalNodeId<mir::Type>,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<()> {
        // mirror the owner's instantiated instance identity
        let symbol = key.symbol;
        let path = self.symbol_path(symbol)?;
        let base = mir::Symbol::declared(self.strings.intern(&path), Self::symbol_identity(symbol));
        let instance = base.instantiate(&key.arguments, tree);

        // receive an exclusive borrow of the dropped storage
        let receiver_storage = nominal_receiver_storage(tree, value);
        let receiver = tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage: receiver_storage,
            access: mir::Access::Exclusive,
            pointee: storage,
            nullability: mir::Nullability::None,
        });

        // declare the extern at the hook's fixed drop shape
        let name = self.qualified_name(symbol.module_id, &path)?;
        let void = tree.intern_type(mir::Type::Void);
        let header = mir::FunctionHeaderBuilder::new(self.strings, &name)
            .arguments(key.arguments.iter().cloned())
            .symbol(instance)
            .parameters(vec![mir::TypeId::from(receiver)])
            .result(void);
        let function = tree.insert(header.imported());

        // record the declaration so later call sites resolve to it
        self.functions
            .insert(key.clone(), FunctionDeclaration::Declared(function));

        Ok(())
    }

    /// Return one imported member's qualified name and receiver type.
    fn imported_member_header(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        member: &ImportedMember,
        signature: dir::FunctionSignatureId,
        owner: ModuleId,
        lifetime_parameters: &LifetimeParameters,
    ) -> CompilerResult<(String, Option<mir::LocalNodeId<mir::Type>>)> {
        // classify the receiver the member declares
        let pointer_bytes = self.pointer_bytes;
        let receiver = match member.role {
            // receive an exclusive borrow of uninitialized storage
            Some(dir::FunctionRole::Constructor) => {
                let owner = self.symbol_type(member.owner)?;
                let value = self
                    .type_lowerer(tree, pointer_bytes, lifetime_parameters)
                    .lower_nominal(owner)?;

                let receiver_storage = nominal_receiver_storage(tree, value.value);

                Some(constructor_receiver_type(
                    tree,
                    value.storage,
                    receiver_storage,
                ))
            }
            // take no receiver for static members
            _ if member.is_static => None,
            // pass this at the declared receiver type
            _ => {
                let declared = self.types(owner)?.signature(signature).this_parameter;
                let Some(declared) = declared else {
                    return Err(CompilerError::Internal {
                        message: "an imported instance method without a receiver".to_string(),
                    });
                };

                Some(
                    self.type_lowerer(tree, pointer_bytes, lifetime_parameters)
                        .lower(declared)?,
                )
            }
        };

        // name the extern after its owner and member
        let name = self.member_extern_name(symbol, member.owner, member.role)?;

        Ok((name, receiver))
    }
    /// Return one imported symbol's member definition, when it names one.
    fn imported_member(
        &self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<ImportedMember>> {
        // find the definition member declared by this symbol
        let state = self.state(symbol.module_id)?;
        for (owner, definition) in state.definitions.iter_definitions() {
            for member in definition.members() {
                if let dir::DefinitionMember::Method(method) = member
                    && method.symbol == symbol
                {
                    return Ok(Some(ImportedMember {
                        owner,
                        role: method.role,
                        is_static: method.space == dir::MemberSpace::Static,
                    }));
                }
            }
        }

        Ok(None)
    }
}
