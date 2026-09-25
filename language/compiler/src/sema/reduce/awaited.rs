use smallvec::SmallVec;
use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::sema::{CheckState, CoroutineBody, CoroutineForm, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Return the completed value carried by one async function result type.
    pub(in crate::sema) fn async_completion_type(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut active = FxIndexSet::default();

        self.async_completion_type_guarded(origin, target, &mut active)
    }

    /// Settle one async completion type, stopping at a transparent cycle.
    fn async_completion_type_guarded(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.normalize(origin, target)?;
        if !active.insert(target) {
            return Ok(None);
        }

        // recognize the two compiler-owned async result representations
        if let dir::Type::Application(instance) = self.ty(target)?
            && matches!(
                self.language_item(instance.symbol)?,
                Some(dir::LanguageItem::Promise | dir::LanguageItem::Task)
            )
        {
            let completed = self.type_id_at(target.module_id, instance.arguments, 0)?;
            active.swap_remove(&target);

            return Ok(completed);
        }

        // preserve transparent aliases and nominal wrappers around an owner
        let backing = if let dir::Type::Application(instance) = self.ty(target)? {
            let definition = self.definition(instance.symbol)?;
            let declared = match definition.as_deref() {
                Some(dir::Definition::TypeAlias(definition)) => Some(definition.value),
                Some(dir::Definition::Newtype(definition)) => Some(definition.backing),
                _ => None,
            };
            if let Some(declared) = declared {
                let substitution = self.instance_substitution(target.module_id, &instance)?;

                Some(self.substitute_type(declared, &substitution)?)
            } else {
                None
            }
        } else {
            None
        };
        let completed = match backing {
            Some(backing) => self.async_completion_type_guarded(origin, backing, active)?,
            None => None,
        };
        active.swap_remove(&target);

        Ok(completed)
    }

    /// Reduce one awaited type through nullish values, unions, and async representations.
    pub(super) fn reduce_awaited(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut active = FxIndexSet::default();

        self.reduce_awaited_guarded(origin, target, &mut active)
    }

    /// Reduce one awaited type with active representation unwrapping tracked.
    fn reduce_awaited_guarded(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // resolve the solved operand before reading its head
        let target = self.shallow_resolve(target)?;
        if !active.insert(target) {
            return Ok(Some(target));
        }

        let reduced = self.reduce_awaited_active(origin, target, active);
        active.swap_remove(&target);

        reduced
    }

    /// Reduce one active awaited target with its reduced head.
    fn reduce_awaited_active(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        active: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // distribute awaitedness over unions
        if let dir::Type::Union(union) = self.ty(target)? {
            let union_elements: SmallVec<[dir::GlobalTypeId; 8]> =
                SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
            let mut elements = Vec::with_capacity(union_elements.len());

            for element in union_elements {
                let Some(element) = self.reduce_awaited_guarded(origin, element, active)? else {
                    return Ok(None);
                };
                elements.push(element);
            }

            let joined = match elements.as_slice() {
                [] => self.intern_type(dir::Type::Never)?,
                [single] => *single,
                _ => self.normalized_union_type(elements)?,
            };

            return Ok(Some(joined));
        }

        // preserve nullish values
        if matches!(
            self.ty(target)?,
            dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::Literal::Null | dir::Literal::Undefined)
        ) {
            return Ok(Some(target));
        }

        // unwrap compiler-recognized async result representations, an object beneath its handle
        let object = self.normalize(origin, target)?;
        let instance = match self.ty(object)? {
            dir::Type::Application(instance) => Some(instance),
            _ => None,
        };
        // read the value the awaited instance yields
        let inner = match instance {
            Some(instance)
                if matches!(
                    self.language_item(instance.symbol)?,
                    Some(dir::LanguageItem::Promise | dir::LanguageItem::Task)
                ) =>
            {
                self.type_id_at(object.module_id, instance.arguments, 0)?
            }
            _ => None,
        };
        if let Some(inner) = inner {
            // a representation unwrap accepts its open payload as final
            let inner = self.shallow_resolve(inner)?;
            if matches!(
                self.ty(inner)?,
                dir::Type::Variable(_) | dir::Type::Parameter(_)
            ) {
                return Ok(Some(inner));
            }

            return self.reduce_awaited_guarded(origin, inner, active);
        }

        // newtypes await through their backing
        if let Some(instance) = self.decompose_newtype(origin, target)? {
            let backing = instance.backing;
            let awaited = self.reduce_awaited_guarded(origin, backing, active)?;

            return match awaited {
                Some(awaited) if awaited != backing => Ok(Some(awaited)),
                _ => Ok(Some(target)),
            };
        }

        // open heads stay stuck until they close
        if matches!(
            self.ty(target)?,
            dir::Type::Variable(_) | dir::Type::Parameter(_)
        ) {
            return Ok(None);
        }

        Ok(Some(target))
    }
}

impl CheckState<'_> {
    /// Record the creation call every checked coroutine body wraps itself in.
    pub(in crate::sema) fn commit_coroutine_creations(&mut self) -> CompilerResult<()> {
        // read the registered coroutine bodies
        let bodies = self.coroutines.clone();

        for body in bodies {
            // anchor the row at the declaring node
            let Some(node) = self.coroutine_declaration_node(body.symbol) else {
                continue;
            };
            if self.decision(node).is_some() {
                continue;
            }

            // leave failed and open targets to the reports they raise
            let mut resolved: SmallVec<[dir::GlobalTypeId; 4]> = SmallVec::new();
            let mut open = false;
            for target in body.form.targets() {
                let target = self.fully_resolve(target)?;
                let flags = self.type_flags(target)?;
                open |= flags.has_error() || flags.has_variable() || flags.has_infer();
                resolved.push(target);
            }
            if open {
                continue;
            }

            // select the creation item the function form names
            let Some(item) = self.coroutine_creation_item(body)? else {
                continue;
            };
            let origin = Origin::Node(node, None);
            let create =
                self.coroutine_creation_call(origin, item, &resolved, TypeSubstitution::default())?;
            self.commit_decision(
                node,
                dir::Decision::Call(dir::OperationResolution::One(create)),
            )?;
        }

        Ok(())
    }

    /// Return the node declaring one coroutine, the anchor of its creation row.
    fn coroutine_declaration_node(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> Option<dir::GlobalNodeIdAny> {
        self.module(symbol.module_id)
            .bindings
            .get_symbol(symbol.local_id)
            .declaration
    }

    /// Return the creation item for one coroutine form, promises split by their written carrier.
    fn coroutine_creation_item(
        &mut self,
        body: CoroutineBody,
    ) -> CompilerResult<Option<dir::LanguageItem>> {
        // name a generator form's creation by asynchrony alone
        if let CoroutineForm::Generator { .. } = body.form {
            return Ok(Some(match body.asynchrony {
                dir::Asynchrony::Sync => dir::LanguageItem::GeneratorCreate,
                dir::Asynchrony::Async => dir::LanguageItem::AsyncGeneratorCreate,
            }));
        }

        // read the carrier an async function names in its declared result
        let Some(declared) = self.adopt_symbol_type_maybe(body.symbol)? else {
            return Ok(None);
        };
        let declared = self.shallow_strip_forms(declared)?;
        let signature = match self.ty(declared)? {
            dir::Type::FunctionSignature(signature) => signature,
            // lambdas declare their signature behind a callable value
            dir::Type::Function(function) => match self.ty(function.signature)? {
                dir::Type::FunctionSignature(signature) => signature,
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let signature = self.type_signature(declared.module_id, signature)?;
        let Some(declared_result) = signature.return_type else {
            return Ok(None);
        };
        let result = self.shallow_strip_forms(declared_result)?;
        let dir::Type::Application(instance) = self.ty(result)? else {
            return Ok(None);
        };

        Ok(match self.language_item(instance.symbol)? {
            Some(dir::LanguageItem::Promise) => Some(dir::LanguageItem::PromiseCreate),
            Some(dir::LanguageItem::Task) => Some(dir::LanguageItem::TaskCreate),
            _ => None,
        })
    }

    /// Record the producer yield call one yield statement runs.
    pub(in crate::sema) fn commit_yield_call(
        &mut self,
        node: dir::GlobalNodeIdAny,
        asynchrony: dir::Asynchrony,
        targets: &[dir::GlobalTypeId],
        value: Option<dir::GlobalNodeIdAny>,
    ) -> CompilerResult<()> {
        if self.decision(node).is_some() {
            return Ok(());
        }

        // select the yield item the generator family names
        let item = match asynchrony {
            dir::Asynchrony::Sync => dir::LanguageItem::GeneratorYield,
            dir::Asynchrony::Async => dir::LanguageItem::AsyncGeneratorYield,
        };
        let origin = self.node_origin(node)?;

        // instantiate the yield at the producer its creation hands the body
        let producer = self.coroutine_producer_type(origin, asynchrony, targets)?;
        let dir::Type::Application(application) = self.ty(producer)? else {
            return Err(CompilerError::Internal {
                message: "a generator producer outside an applied struct".to_string(),
            });
        };
        let arguments = self
            .type_ids(producer.module_id, application.arguments)?
            .to_vec();
        let symbol = self.language_symbol(item)?;
        let Some(template) = self.symbol_template(symbol)? else {
            return Err(CompilerError::Internal {
                message: "a yield method without a template".to_string(),
            });
        };
        let owners = self.owner_template_parameters(template)?;
        let bound = TypeSubstitution {
            bindings: owners
                .iter()
                .zip(arguments)
                .map(|(parameter, argument)| dir::GenericArgumentBinding::new(*parameter, argument))
                .collect(),
            receiver: Some(producer),
        };
        let mut call = self.coroutine_creation_call(origin, item, &[], bound)?;

        // bind the yielded value as the provided argument
        if let Some(value) = value
            && let Some(binding) = call.arguments.last_mut()
        {
            binding.source = dir::ArgumentSource::Provided(value);
        }

        self.commit_decision(
            node,
            dir::Decision::Call(dir::OperationResolution::One(call)),
        )
    }

    /// Return the producer type one generator creation hands its body closure.
    fn coroutine_producer_type(
        &mut self,
        origin: Origin,
        asynchrony: dir::Asynchrony,
        targets: &[dir::GlobalTypeId],
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the body closure argument of the creation call
        let item = match asynchrony {
            dir::Asynchrony::Sync => dir::LanguageItem::GeneratorCreate,
            dir::Asynchrony::Async => dir::LanguageItem::AsyncGeneratorCreate,
        };
        let create =
            self.coroutine_creation_call(origin, item, targets, TypeSubstitution::default())?;
        let Some(slot) = create
            .arguments
            .iter()
            .find(|binding| matches!(binding.source, dir::ArgumentSource::Supplied(_)))
        else {
            return Err(CompilerError::Internal {
                message: "a generator creation without its body closure slot".to_string(),
            });
        };

        // strip the body closure down to its signature
        let closure = self.normalize(origin, slot.parameter_type)?;
        let closure = self.shallow_strip_forms(closure)?;
        let Some((module, head)) = self.callable_signature_head(closure)? else {
            return Err(CompilerError::Internal {
                message: "a generator body slot outside a callable".to_string(),
            });
        };

        // take the closure's first parameter, the producer
        let parameters = self.signature_parameters(module, head.parameters)?;
        let Some(producer) = parameters.first().map(|parameter| parameter.ty) else {
            return Err(CompilerError::Internal {
                message: "a generator body slot without its producer parameter".to_string(),
            });
        };

        Ok(producer)
    }

    /// Return the call creating one coroutine object through its language item.
    fn coroutine_creation_call(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        targets: &[dir::GlobalTypeId],
        bound: TypeSubstitution,
    ) -> CompilerResult<dir::Call> {
        let symbol = self.language_symbol(item)?;
        let mut resolved = SmallVec::<[dir::GlobalTypeId; 4]>::with_capacity(targets.len());
        for target in targets {
            resolved.push(self.fully_resolve(*target)?);
        }
        let Some(call) = self.instantiate_symbol_call(origin, symbol, &resolved, bound)? else {
            return Err(CompilerError::Internal {
                message: "a coroutine creation item declares no callable".to_string(),
            });
        };

        Ok(call)
    }
}
