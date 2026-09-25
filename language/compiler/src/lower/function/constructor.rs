use tspp_dir as dir;
use tspp_dir::TypeVisit;
use tspp_mir as mir;

use crate::lower::function::argument::Argument;
use crate::lower::{
    Body, BoundReceiver, FunctionDefinition, FunctionLowerer, GenericScope, Instance, ModuleLowerer,
};
use crate::{CompilerError, CompilerResult};

impl FunctionLowerer<'_, '_, '_> {
    /// Bind the generated allocating entry of one selected constructor.
    pub(in crate::lower) fn lower_constructor_value(
        &mut self,
        target: dir::GlobalTypeId,
        construction: &dir::ConstructDecision,
    ) -> CompilerResult<mir::Value> {
        // identify the selected constructor and its applied class
        let dir::ConstructTarget::Class {
            key,
            constructor,
            arguments: bindings,
        } = &construction.target
        else {
            return Err(self.internal("a constructor function outside class construction"));
        };
        let (symbol, bindings) = match constructor.call_symbol() {
            Some(symbol) => (symbol, bindings),
            None => (key.symbol, &key.arguments),
        };
        let arguments: Vec<_> = bindings.iter().map(|binding| binding.argument).collect();

        // capture the generic parameters used by the signature and construction
        let (signature, owner) = self.lower.signature(target)?;
        let signature = *self.lower.types(owner)?.signature(signature);
        let mut scope = GenericScope::for_signature(self.lower, signature.template, None)?;
        let mut types = vec![target];
        construction.visit_types(&mut |ty| {
            types.push(ty);
            Ok::<_, CompilerError>(())
        })?;
        scope.capture(types, &self.scope, self.lower)?;

        // identify and lower the entry under the same parameter indices as its body
        let instance = self
            .lower
            .type_lowerer(self.builder.tree_mut(), &scope)
            .generic_instance_key(symbol, Some(construction.return_type), &arguments)?;
        let signature_type = self
            .lower
            .type_lowerer(self.builder.tree_mut(), &self.scope)
            .lower_bare_signature(&signature, owner, None, None)?;

        // identify the complete generated declaration, including its parameter domains
        let tree = self.builder.tree_mut();
        let generics = self
            .lower
            .generic_parameters(tree, &scope, BoundReceiver::None)?;
        let name = format!("{}.new", self.lower.symbol_path(symbol)?);
        let base = mir::Symbol::declared(
            self.lower.module,
            self.lower.strings.intern(&name),
            ModuleLowerer::symbol_identity(symbol),
        );
        let identity = mir::Symbol::generated(
            base,
            signature_type,
            &generics,
            instance.receiver.as_ref(),
            &instance.arguments,
            tree,
        );

        // apply the construction's bindings in the generated function's parameter order,
        //  a captured parameter bound at itself
        let mut arguments = vec![None; scope.count() as usize];
        for (parameter, index) in &scope.parameters {
            let bound = bindings
                .iter()
                .find(|binding| binding.parameter == *parameter)
                .map(|binding| binding.argument);
            let ty = match bound {
                Some(argument) => argument,
                None => {
                    self.lower
                        .state(parameter.module_id)?
                        .generics
                        .get_parameter(parameter.local_id)
                        .ty
                }
            };
            arguments[*index as usize] = Some(self.lower_generic_argument(ty)?);
        }
        for dependent in scope.dependents.values() {
            arguments[dependent.index as usize] = Some(self.lower_generic_argument(dependent.ty)?);
        }
        let arguments = arguments
            .into_iter()
            .map(|argument| {
                argument.ok_or_else(|| {
                    self.internal("a constructor function with an unbound generic parameter")
                })
            })
            .collect::<CompilerResult<Vec<_>>>()?;

        // declare each constructor entry once at its callable signature
        let function = if let Some(function) = self.lower.constructors.get(&identity) {
            *function
        } else {
            // lower the entry's signature under its own parameters
            let tree = self.builder.tree_mut();
            let (parameters, result) = self.lower.lower_signature(tree, target, &scope)?;

            // declare the entry with its referenced generic parameters
            let header =
                mir::FunctionHeaderBuilder::new(self.lower.strings, self.lower.module, &name)
                    .symbol(identity)
                    .generics(generics);
            let header = scope.declare(header).parameters(parameters).result(result);
            let function = tree.insert(header.declared());
            self.lower.anchor_function(tree, function, symbol)?;

            // register the entry before scheduling its construction body
            self.lower.constructors.insert(identity, function);
            self.lower.pending.push(FunctionDefinition {
                function,
                symbol,
                has_this: false,
                parameters: Vec::new(),
                scope,
                source: self.source,
                constructs: None,
                defaults: Vec::new(),
                body: Body::Constructor(Box::new(construction.clone())),
            });

            function
        };

        // bind the generated function at the caller's type arguments
        let instance = match arguments.is_empty() {
            true => Instance::Declared(function),
            false => Instance::Applied {
                template: function,
                arguments,
            },
        };

        // use the ordinary function value representation
        let ty = self.lower_type(target)?;
        self.bind_function_value(ty, instance, None)?
            .ok_or_else(|| {
                self.internal("a constructor function without a callable representation")
            })
    }

    /// Construct and return an instance from a generated function's parameters.
    pub(in crate::lower) fn lower_constructor_body(
        &mut self,
        construction: &dir::ConstructDecision,
    ) -> CompilerResult<()> {
        // supply the generated parameters to ordinary class construction
        let count = self
            .builder
            .tree()
            .get(self.builder.function_id())
            .parameters
            .len();
        let arguments: Vec<_> = (0..count)
            .map(|index| Argument::Value(self.builder.function_parameter(index)))
            .collect();
        let value = self.lower_construction(construction, &arguments)?;

        self.return_value(Some(value))
    }
}
