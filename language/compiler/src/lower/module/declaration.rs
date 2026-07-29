use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use smallvec::SmallVec;

use crate::lower::{FunctionDefinition, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// The callables one root declaration contributes.
enum RootCallables {
    /// One function body to declare.
    Function(dir::LocalNodeId<dir::Expression>),
    /// One member list to declare under its host declaration.
    Members(SmallVec<[dir::LocalNodeId<dir::Member>; 8]>),
    /// No runtime code.
    Inert,
}

impl ModuleLowerer<'_> {
    /// Declare the runtime callables of one root declaration.
    pub(in crate::lower) fn declare_root(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        bodies: &mut Vec<FunctionDefinition>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // classify the declaration in one narrow tree borrow
        let runtime = match self.local().tree().get(declaration) {
            // function f() { ... }
            dir::Declaration::Function(function) => match function.body {
                Some(body) => RootCallables::Function(body),
                // ambient functions declare no body
                None => RootCallables::Inert,
            },

            // members declare their own callables
            dir::Declaration::Class(class) => {
                RootCallables::Members(SmallVec::from_slice(&class.members))
            }
            dir::Declaration::Struct(structure) => {
                RootCallables::Members(SmallVec::from_slice(&structure.members))
            }
            dir::Declaration::Enum(enumeration) => {
                RootCallables::Members(SmallVec::from_slice(&enumeration.members))
            }
            dir::Declaration::Extension(extension) => {
                RootCallables::Members(SmallVec::from_slice(&extension.members))
            }

            // interface members declare signatures only
            dir::Declaration::Interface(_) => RootCallables::Inert,
            // type declarations erase at runtime
            dir::Declaration::Type(_) => RootCallables::Inert,
            // ambient global and module blocks declare no runtime code
            dir::Declaration::Global(_) | dir::Declaration::Module(_) => RootCallables::Inert,
        };

        match runtime {
            RootCallables::Function(body) => {
                // defer generic functions to their concrete instances
                let node = declaration.into_global_any(self.module);
                if let Some(symbol) = self.symbol_declared_at(node)?
                    && self.signature_has_instance_parameters(self.symbol_type(symbol)?)?
                {
                    return Ok(());
                }

                // accumulate unsupported diagnostics; abort on internal failures
                match self.declare_function(builder, declaration, body) {
                    Ok(body) => bodies.push(body),
                    Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
                    Err(error) => return Err(error),
                }

                Ok(())
            }
            RootCallables::Members(members) => {
                self.declare_members(builder, declaration, &members, bodies, errors)
            }
            RootCallables::Inert => Ok(()),
        }
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

        for member in members {
            // accumulate unsupported diagnostics; abort on internal failures
            match self.declare_member(builder, owner_symbol, *member, bodies) {
                Ok(()) => {}
                Err(CompilerError::Diagnostic(diagnostic)) => errors.push(diagnostic),
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

            // data and type members declare no code of their own
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
            dir::Member::ComptimeBlock { .. } => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a comptime block".to_string(),
                }
                .into());
            }
            dir::Member::Error => {
                return Err(CompilerError::Internal {
                    message: "checked DIR retained a malformed member".to_string(),
                });
            }
        };

        // bodiless members are ambient or abstract
        let Some(body) = body else {
            return Ok(());
        };

        match role {
            // methods, constructors and accessors all lower as callables
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

        let Some(owner_symbol) = owner_symbol else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one member owner".to_string(),
            });
        };

        // parameterized owners lower their members per concrete instance
        let is_parameterized = match self.definition(owner_symbol)? {
            Some(definition) => {
                self.definition_is_parameterized(owner_symbol.module_id, definition)?
            }
            None => false,
        };
        if is_parameterized {
            return Ok(());
        }

        // read the member symbol from the checked definition
        let node = member.into_global_any(self.module);
        let Some(symbol) = self.method_symbol(owner_symbol, node)? else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one method declaration".to_string(),
            });
        };

        // defer generic members to their concrete instances
        if self.signature_has_instance_parameters(self.symbol_type(symbol)?)? {
            return Ok(());
        }

        // declare the header and queue its body
        bodies.push(self.declare_method(builder, owner_symbol, symbol, member, body, is_static)?);

        Ok(())
    }
}
