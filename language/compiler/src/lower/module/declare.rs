use smallvec::SmallVec;
use tspp_artifact::DiagnosticLike;
use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{
    FunctionDeclaration, FunctionDefinition, GenericInstanceKey, GenericScope, LowerPhase,
    ModuleInitializer, ModuleLowerer,
};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Declare every identity the module's bodies build against.
    pub(in crate::lower) fn declare_module(
        &mut self,
        tree: &mut mir::Tree,
    ) -> CompilerResult<(Vec<FunctionDefinition>, Vec<Box<dyn DiagnosticLike>>)> {
        let mut errors = Vec::new();

        // lower every concrete nominal declaration owned by this module
        self.lower_nominal_declarations(tree, &mut errors)?;

        // define the synthesized constructors beside their class declarations
        self.declare_default_constructors(tree, &mut errors)?;

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
                // run every other root as a module initializer statement
                _ => {
                    self.initializers.push(ModuleInitializer::Statement(root));

                    continue;
                }
            };

            // declare the callables of the root declaration
            self.declare_root(tree, declaration, &mut bodies, &mut errors)?;
        }

        // declare the polymorphic function of every template
        let templates = self.declare_templates(tree, &mut errors)?;
        bodies.extend(templates);

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

        // declare one function body, polymorphic over its template parameters
        let declares_header = body.is_some()
            || (self.phase == LowerPhase::Declare
                && matches!(
                    self.local().tree().get(declaration),
                    dir::Declaration::Function(_)
                ));
        if declares_header {
            let node = declaration.into_global_any(self.module);

            let Some(symbol) = self.symbol_declared_at(node)? else {
                return Err(CompilerError::Internal {
                    message: "a missing symbol for one function declaration".to_string(),
                });
            };

            // leave a template to its polymorphic declaration
            let chain = self.callable_scope(symbol, None, false, None)?;
            if chain.count() > 0 {
                return Ok(());
            }

            // accumulate unsupported diagnostics; abort on internal failures
            let key = GenericInstanceKey::non_generic(symbol);
            match self.declare_callable(tree, &key, &GenericScope::default()) {
                Ok(Some(body)) => bodies.push(body),
                Ok(None) => {}
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

        // declare each member of the owner, accumulating unsupported diagnostics
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

    /// Declare one member's callable, when it has runtime code.
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

            // declare the global behind an associated const
            dir::Member::AssociatedConst { value: Some(_), .. } => {
                return self.declare_associated_const(tree, member);
            }

            // skip field, type, and requirement members
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

        // skip a member the declaration leaves bodiless
        if body.is_none() {
            return Ok(());
        }

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

        // read the member symbol from its definition
        let node = member.into_global_any(self.module);
        let Some(symbol) = self.method_symbol(owner_symbol, node)? else {
            return Err(CompilerError::Internal {
                message: format!("a missing symbol for the method at {node:?} on {owner_symbol:?}"),
            });
        };

        // leave a template to its polymorphic declaration
        let chain = self.callable_scope(symbol, Some(owner_symbol), is_static, None)?;
        if chain.count() > 0 {
            return Ok(());
        }

        // declare the header and queue its body
        let key = GenericInstanceKey::non_generic(symbol);
        if let Some(definition) = self.declare_callable(tree, &key, &GenericScope::default())? {
            bodies.push(definition);
        }

        Ok(())
    }
}
