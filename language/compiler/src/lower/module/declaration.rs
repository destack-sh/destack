use destack_artifact::DiagnosticLike;
use destack_dir as dir;
use destack_mir as mir;
use smallvec::SmallVec;

use crate::lower::{Body, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// The runtime work one root declaration contributes.
enum Declared {
    /// One function body to declare.
    Function(dir::LocalNodeId<dir::Expression>),
    /// One member list to declare under its host declaration.
    Members(MemberHost, SmallVec<[dir::LocalNodeId<dir::Member>; 8]>),
    /// No runtime code.
    Inert,
}

/// The declaration family hosting one member list.
#[derive(Clone, Copy)]
enum MemberHost {
    Class,
    Struct,
    Enum,
    Extension,
}

impl ModuleLowerer<'_> {
    /// Declare the runtime callables of one root declaration.
    ///
    /// Every declaration kind classifies explicitly: type-level forms are
    /// inert, generic templates wait for their materialized instances, and
    /// pending forms fail loudly at their declaration.
    pub(in crate::lower) fn lower_declaration(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        bodies: &mut Vec<Body>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // classify the declaration in one narrow tree borrow
        let declared = match self.local().tree().get(declaration) {
            // function f() { ... }
            dir::Declaration::Function(function) => match function.body {
                Some(body) => Declared::Function(body),
                // ambient functions declare no body
                None => Declared::Inert,
            },

            // members declare their own callables
            dir::Declaration::Class(class) => {
                Declared::Members(MemberHost::Class, SmallVec::from_slice(&class.members))
            }
            dir::Declaration::Struct(structure) => {
                Declared::Members(MemberHost::Struct, SmallVec::from_slice(&structure.members))
            }
            dir::Declaration::Enum(enumeration) => {
                Declared::Members(MemberHost::Enum, SmallVec::from_slice(&enumeration.members))
            }
            dir::Declaration::Extension(extension) => Declared::Members(
                MemberHost::Extension,
                SmallVec::from_slice(&extension.members),
            ),

            // interface members declare signatures only
            dir::Declaration::Interface(_) => Declared::Inert,
            // type aliases erase at runtime
            dir::Declaration::Type(_) => Declared::Inert,
            // ambient global and module blocks declare no runtime code
            dir::Declaration::Global(_) | dir::Declaration::Module(_) => Declared::Inert,
        };

        match declared {
            Declared::Function(body) => {
                // generic functions defer to their materialized instances
                let node = declaration.into_global_any(self.module);
                if let Some(symbol) = self.symbol_declared_at(node)?
                    && self.signature_is_generic(self.symbol_type(symbol)?)?
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
            Declared::Members(host, members) => {
                self.declare_members(builder, declaration, host, &members, bodies, errors)
            }
            Declared::Inert => Ok(()),
        }
    }

    /// Declare the callable members of one member-bearing declaration.
    fn declare_members(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        host: MemberHost,
        members: &[dir::LocalNodeId<dir::Member>],
        bodies: &mut Vec<Body>,
        errors: &mut Vec<Box<dyn DiagnosticLike>>,
    ) -> CompilerResult<()> {
        // resolve the host symbol owning the member namespace
        let node = declaration.into_global_any(self.module);
        let owner = self.symbol_declared_at(node)?;

        for member in members {
            // accumulate unsupported diagnostics; abort on internal failures
            match self.declare_member(builder, host, owner, *member, bodies) {
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
        host: MemberHost,
        owner: Option<dir::GlobalSymbolId>,
        member: dir::LocalNodeId<dir::Member>,
        bodies: &mut Vec<Body>,
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
        if is_static {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a static method".to_string(),
            }
            .into());
        }
        match role {
            None | Some(dir::FunctionRole::Constructor) => {}
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter) => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "an accessor member".to_string(),
                }
                .into());
            }
            Some(dir::FunctionRole::New | dir::FunctionRole::Call) => {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "a callable role member".to_string(),
                }
                .into());
            }
        }

        // instance methods receive this at their sealed receiver type
        match host {
            MemberHost::Class | MemberHost::Struct | MemberHost::Enum => {
                let Some(owner) = owner else {
                    return Err(CompilerError::Internal {
                        message: "checked DIR is missing a symbol for one nominal".to_string(),
                    });
                };

                // generic nominal templates wait for their instances
                if self
                    .definition(owner)?
                    .and_then(dir::Definition::template)
                    .is_some()
                {
                    return Ok(());
                }
                bodies.push(self.declare_method(builder, owner.local_id, member, body)?);

                Ok(())
            }
            MemberHost::Extension => Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "an extension method".to_string(),
            }
            .into()),
        }
    }
}
