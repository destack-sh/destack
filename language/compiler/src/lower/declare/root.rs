use std::sync::Arc;

use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use smallvec::SmallVec;

use crate::lower::{FunctionDeclaration, FunctionDefinition, GenericInstanceKey, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Declare every identity the module's bodies build against.
    ///
    /// Returns the queued function definitions and the recoverable errors.
    pub(in crate::lower) fn declare_module(
        &mut self,
        builder: &mut mir::ModuleBuilder,
    ) -> CompilerResult<(Vec<FunctionDefinition>, Vec<Box<dyn DiagnosticLike>>)> {
        // lower every concrete nominal declaration owned by this module
        let mut errors = Vec::new();
        self.lower_nominal_declarations(builder)?;

        // declare every callable header so bodies can call in any order
        let mut bodies = Vec::new();
        for index in 0..self.local().roots.len() {
            let root = self.local().roots[index];

            // classify the root form
            let declaration = match *self.local().tree().get(root) {
                dir::Expression::Declaration(declaration) => declaration,
                // skip imports and exports
                dir::Expression::Import { .. } | dir::Expression::Export { .. } => continue,
                // declare the evaluated globals of module constants
                dir::Expression::Let {
                    mutability,
                    ref declarators,
                    ..
                } => {
                    let declarators = declarators.clone();
                    match self.declare_module_constants(builder, mutability, &declarators) {
                        Ok(()) => {}
                        Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                        Err(error) => return Err(error),
                    }

                    continue;
                }
                ref other => {
                    let error = LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: format!("a module-level '{}' statement", other.variant_name()),
                    };
                    match CompilerError::from(error) {
                        CompilerError::Diagnostic(diagnostic) => errors.push(diagnostic),
                        error => return Err(error),
                    }

                    continue;
                }
            };
            self.declare_root(builder, declaration, &mut bodies, &mut errors)?;
        }

        // declare every concrete generic instance reachable from a body
        let (instances, reachable) =
            self.declare_reachable_instances(builder, &bodies, &mut errors)?;
        bodies.extend(instances);

        // declare an import for every foreign callable the bodies call
        for symbol in reachable.imports {
            match self.declare_imported_function(builder, symbol) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, &mut errors);
                }
                Err(error) => return Err(error),
            }
        }

        // declare a dotted host extern for every binding the bodies call
        for symbol in reachable.bindings {
            match self.declare_binding_function(builder, symbol) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bank_failed_callable(Some(symbol), diagnostic, &mut errors);
                }
                Err(error) => return Err(error),
            }
        }

        // declare an imported global for every foreign constant the bodies read
        for symbol in reachable.constants {
            match self.declare_imported_constant(builder, symbol) {
                Ok(()) => {}
                // keep the diagnostic for the first body that reads the constant
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.globals.insert(symbol, Err(Arc::from(diagnostic)));
                }
                Err(error) => return Err(error),
            }
        }

        // declare the dispatch entries behind every collected implementer
        for (source, target) in reachable.implementers {
            match self.declare_implementer(builder, source, target) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                Err(error) => return Err(error),
            }
        }

        // declare the immortal String and BigInt objects behind every collected literal
        self.declare_string_literals(builder, reachable.strings)?;
        self.declare_bigint_literals(builder, reachable.bigints)?;

        // declare every closure the bodies bind, queueing each for lowering
        for (declaration, body) in reachable.closures {
            match self.declare_function(builder, declaration, body) {
                Ok(closure) => bodies.push(closure),
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    let node = declaration.into_global_any(self.module);
                    let symbol = self.symbol_declared_at(node)?;
                    self.bank_failed_callable(symbol, diagnostic, &mut errors);
                }
                Err(error) => return Err(error),
            }
        }

        Ok((bodies, errors))
    }

    /// Declare the runtime callables of one root declaration.
    pub(in crate::lower) fn declare_root(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        bodies: &mut Vec<FunctionDefinition>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // read the declared body or member list in one narrow tree borrow
        let (body, members) = match self.local().tree().get(declaration) {
            // function f() { ... }, skipping ambient signatures
            dir::Declaration::Function(function) => (function.body, SmallVec::new()),

            // members declare their own callables
            dir::Declaration::Class(class) => (None, SmallVec::from_slice(&class.members)),
            dir::Declaration::Struct(structure) => (None, SmallVec::from_slice(&structure.members)),
            dir::Declaration::Enum(enumeration) => {
                (None, SmallVec::from_slice(&enumeration.members))
            }
            dir::Declaration::Extension(extension) => {
                (None, SmallVec::from_slice(&extension.members))
            }

            // interfaces, type declarations, and ambient blocks carry no runtime code
            dir::Declaration::Interface(_)
            | dir::Declaration::Type(_)
            | dir::Declaration::Global(_)
            | dir::Declaration::Module(_) => (None, SmallVec::<[_; 8]>::new()),
        };

        // declare one function body, deferring generics to their instances
        if let Some(body) = body {
            let node = declaration.into_global_any(self.module);
            if let Some(symbol) = self.symbol_declared_at(node)?
                && self.signature_has_instance_parameters(self.symbol_type(symbol)?)?
            {
                return Ok(());
            }

            // accumulate unsupported diagnostics; abort on internal failures
            match self.declare_function(builder, declaration, body) {
                Ok(body) => bodies.push(body),
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    let symbol = self.symbol_declared_at(node)?;
                    self.bank_failed_callable(symbol, diagnostic, errors);
                }
                Err(error) => return Err(error),
            }

            return Ok(());
        }

        self.declare_members(builder, declaration, &members, bodies, errors)
    }

    /// Record one failed callable declaration and keep its diagnostic.
    pub(in crate::lower) fn bank_failed_callable(
        &mut self,
        symbol: Option<dir::GlobalSymbolId>,
        diagnostic: Box<dyn DiagnosticLike>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) {
        // keep any declared function over the failure marker
        if let Some(symbol) = symbol {
            self.functions
                .entry(GenericInstanceKey::non_generic(symbol))
                .or_insert(FunctionDeclaration::Failed);
        }
        errors.push(diagnostic);
    }

    /// Declare the callable members of one member-bearing declaration.
    fn declare_members(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        members: &[dir::LocalNodeId<dir::Member>],
        bodies: &mut Vec<FunctionDefinition>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // resolve the declaration symbol owning the member namespace
        let node = declaration.into_global_any(self.module);
        let owner_symbol = self.symbol_declared_at(node)?;

        // generic owners declare their members per concrete instance in declare_reachable_instances
        if let Some(owner) = owner_symbol
            && self.owner_has_instance_parameters(owner)?
        {
            return Ok(());
        }

        // declare each member of the concrete owner, accumulating unsupported
        //  diagnostics and aborting on internal failures
        for member in members {
            match self.declare_member(builder, owner_symbol, *member, bodies) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    let node = member.into_global_any(self.module);
                    let symbol = self.symbol_declared_at(node)?;
                    self.bank_failed_callable(symbol, diagnostic, errors);
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare one member's callable, when it carries runtime code.
    fn declare_member(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        owner_symbol: Option<dir::GlobalSymbolId>,
        member: dir::LocalNodeId<dir::Member>,
        bodies: &mut Vec<FunctionDefinition>,
    ) -> CompilerResult<()> {
        // classify the member in one narrow tree borrow
        let (role, is_static, body) = match self.local().tree().get(member) {
            dir::Member::Method {
                signature,
                body,
                is_static,
                ..
            } => (signature.role, *is_static, *body),

            // skip data and type members
            dir::Member::Field { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => return Ok(()),

            dir::Member::StaticBlock { .. } => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a static initialization block".to_string(),
                }
                .into());
            }
            dir::Member::ConstBlock { .. } => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a const block".to_string(),
                }
                .into());
            }
            dir::Member::Error => {
                return Err(CompilerError::Internal {
                    message: "a malformed member".to_string(),
                });
            }
        };

        // skip bodiless members
        let Some(body) = body else {
            return Ok(());
        };

        // reject the roles without a runtime callable
        match role {
            // accept methods, constructors and accessors
            None
            | Some(
                dir::FunctionRole::Constructor
                | dir::FunctionRole::Getter
                | dir::FunctionRole::Setter,
            ) => {}
            Some(dir::FunctionRole::New | dir::FunctionRole::Call) => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a callable role member".to_string(),
                }
                .into());
            }
        }

        // members always declare inside a named owner
        let Some(owner_symbol) = owner_symbol else {
            return Err(CompilerError::Internal {
                message: "missing a symbol for one member owner".to_string(),
            });
        };

        // parameterized owners declare their members per concrete instance in
        //  declare_reachable_instances
        let is_parameterized = match self.definition(owner_symbol)? {
            Some(definition) => {
                self.definition_is_parameterized(owner_symbol.module_id, definition)?
            }
            None => false,
        };
        if is_parameterized {
            return Ok(());
        }

        // read the member symbol from its definition
        let node = member.into_global_any(self.module);
        let Some(symbol) = self.method_symbol(owner_symbol, node)? else {
            return Err(CompilerError::Internal {
                message: "missing a symbol for one method declaration".to_string(),
            });
        };

        // generic members declare per concrete instance in declare_reachable_instances
        if self.signature_has_instance_parameters(self.symbol_type(symbol)?)? {
            return Ok(());
        }

        // declare the header and queue its body
        bodies.push(self.declare_method(builder, owner_symbol, symbol, member, body, is_static)?);

        Ok(())
    }
}
