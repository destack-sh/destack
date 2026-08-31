use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use smallvec::SmallVec;

use crate::lower::{FunctionDeclaration, FunctionDefinition, GenericInstanceKey, LowerState};
use crate::{CompilerError, CompilerResult, LowerError};

impl LowerState<'_> {
    /// Declare every identity the module's bodies build against.
    pub(in crate::lower) fn declare_module(
        &mut self,
        tree: &mut mir::Tree,
    ) -> CompilerResult<(Vec<FunctionDefinition>, Vec<Box<dyn DiagnosticLike>>)> {
        let mut errors = Vec::new();

        // lower every concrete nominal declaration owned by this module
        self.lower_nominal_declarations(tree)?;

        // define the synthesized constructors beside their class declarations
        self.declare_default_constructors(tree)?;

        // declare every callable header ahead of the bodies
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
                    match self.declare_module_constants(tree, mutability, &declarators) {
                        Ok(()) => {}
                        Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                        Err(error) => return Err(error),
                    }

                    continue;
                }
                // report every other module-level statement
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

            // declare the callables of the root declaration
            self.declare_root(tree, declaration, &mut bodies, &mut errors)?;
        }

        // declare every closed callable instance
        let instances = self.declare_instances(tree, &mut errors)?;
        bodies.extend(instances);

        Ok((bodies, errors))
    }

    /// Declare the runtime callables of one root declaration.
    fn declare_root(
        &mut self,
        tree: &mut mir::Tree,
        declaration: dir::LocalNodeId<dir::Declaration>,
        bodies: &mut Vec<FunctionDefinition>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // read the declared body or member list in one narrow tree borrow
        let (body, members) = match self.local().tree().get(declaration) {
            // take the body of a function declaration, absent on an ambient signature
            dir::Declaration::Function(function) => (function.body, SmallVec::new()),

            // take the member list of a member-bearing declaration
            dir::Declaration::Class(class) => (None, SmallVec::from_slice(&class.members)),
            dir::Declaration::Struct(structure) => (None, SmallVec::from_slice(&structure.members)),
            dir::Declaration::Enum(enumeration) => {
                (None, SmallVec::from_slice(&enumeration.members))
            }
            dir::Declaration::Extension(extension) => {
                (None, SmallVec::from_slice(&extension.members))
            }

            // skip the declarations without runtime code
            dir::Declaration::Interface(_)
            | dir::Declaration::Type(_)
            | dir::Declaration::Global(_)
            | dir::Declaration::Module(_) => (None, SmallVec::<[_; 8]>::new()),
        };

        // declare one function body, deferring generics to their instances
        if let Some(body) = body {
            // defer generic functions to declare_instances
            let node = declaration.into_global_any(self.module);
            if let Some(symbol) = self.symbol_declared_at(node)?
                && self.signature_has_parameters_beyond_extents(self.symbol_type(symbol)?)?
            {
                return Ok(());
            }

            // accumulate unsupported diagnostics; abort on internal failures
            match self.declare_function(tree, declaration, body) {
                Ok(body) => bodies.push(body),
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    let symbol = self.symbol_declared_at(node)?;
                    self.bank_failed_callable(symbol, diagnostic, errors);
                }
                Err(error) => return Err(error),
            }

            return Ok(());
        }

        self.declare_members(tree, declaration, &members, bodies, errors)
    }

    /// Mark one callable failed and bank its diagnostic.
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
        tree: &mut mir::Tree,
        declaration: dir::LocalNodeId<dir::Declaration>,
        members: &[dir::LocalNodeId<dir::Member>],
        bodies: &mut Vec<FunctionDefinition>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // resolve the declaration symbol owning the member namespace
        let node = declaration.into_global_any(self.module);
        let owner_symbol = self.symbol_declared_at(node)?;

        // defer the members of generic owners to declare_instances
        if let Some(owner) = owner_symbol
            && self.owner_has_instance_parameters(owner)?
        {
            return Ok(());
        }

        // declare each member of the concrete owner, accumulating unsupported
        //  diagnostics and aborting on internal failures
        for member in members {
            match self.declare_member(tree, owner_symbol, *member, bodies) {
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
        tree: &mut mir::Tree,
        owner_symbol: Option<dir::GlobalSymbolId>,
        member: dir::LocalNodeId<dir::Member>,
        bodies: &mut Vec<FunctionDefinition>,
    ) -> CompilerResult<()> {
        // skip members whose static gates decided absence
        let presence = self.state(self.module)?.statics.presence(member.into_any());
        if presence == Some(dir::StaticPresence::Absent) {
            return Ok(());
        }

        // classify the member in one narrow tree borrow
        let (role, is_static, body) = match self.local().tree().get(member) {
            dir::Member::Method {
                signature,
                body,
                is_static,
                ..
            } => (signature.role, *is_static, *body),

            // skip field and type members
            dir::Member::Field { .. }
            | dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. } => return Ok(()),

            // reject a static initialization block
            dir::Member::StaticBlock { .. } => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a static initialization block".to_string(),
                }
                .into());
            }
            // reject a const block
            dir::Member::ConstBlock { .. } => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a const block".to_string(),
                }
                .into());
            }
            // reject a malformed member
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
            // reject the callable roles
            Some(dir::FunctionRole::New | dir::FunctionRole::Call) => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a callable role member".to_string(),
                }
                .into());
            }
        }

        // require a named owner for every member
        let Some(owner_symbol) = owner_symbol else {
            return Err(CompilerError::Internal {
                message: "a missing symbol for one member owner".to_string(),
            });
        };

        // defer the members of parameterized owners to declare_instances
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
                message: format!("a missing symbol for the method at {node:?} on {owner_symbol:?}"),
            });
        };

        // defer generic members to declare_instances
        if self.signature_has_parameters_beyond_extents(self.symbol_type(symbol)?)? {
            return Ok(());
        }

        // declare the header and queue its body
        bodies.push(self.declare_method(tree, owner_symbol, symbol, member, body, is_static)?);

        Ok(())
    }
}
