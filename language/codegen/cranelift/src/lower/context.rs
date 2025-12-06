use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::Context;
use cranelift_codegen::ir::Function as CraneliftFunction;
use cranelift_codegen::isa::TargetIsa;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use destack_mir::{Function, LocalNodeId, NodeTree};
use destack_source::ImmutableStringPool;

use super::FunctionLowerer;
use crate::CraneliftError;

/// Context for lowering an entire MIR module to Cranelift.
///
/// Manages the Cranelift module, function declarations, and lowering state.
pub(crate) struct ModuleLowerer<'a> {
    /// The target ISA.
    isa: Arc<dyn TargetIsa>,
    /// String pool for resolving names.
    strings: &'a ImmutableStringPool,
    /// The Cranelift object module.
    module: ObjectModule,
    /// Mapping from MIR function ids to Cranelift function ids.
    function_ids: HashMap<LocalNodeId<Function>, FuncId>,
    /// Compiled Cranelift functions (for CLIF output).
    compiled_functions: Vec<(String, CraneliftFunction)>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(isa: &'a dyn TargetIsa, strings: &'a ImmutableStringPool) -> Self {
        // we need to clone the isa for ObjectBuilder - it takes Arc
        // for now, re-lookup the isa (this is a simplification)
        let isa_arc: Arc<dyn TargetIsa> = isa.triple().clone().pipe(|triple| {
            let flags = isa.flags().clone();
            cranelift_codegen::isa::lookup(triple)
                .expect("isa lookup failed")
                .finish(flags)
                .expect("isa finish failed")
        });

        let builder = ObjectBuilder::new(
            isa_arc.clone(),
            "module",
            cranelift_module::default_libcall_names(),
        )
        .expect("failed to create object builder");
        let module = ObjectModule::new(builder);

        Self {
            isa: isa_arc,
            strings,
            module,
            function_ids: HashMap::new(),
            compiled_functions: Vec::new(),
        }
    }

    /// Lower an entire MIR module.
    pub(crate) fn lower_module(&mut self, tree: &NodeTree) -> Result<(), CraneliftError> {
        // first pass: declare all functions
        self.declare_functions(tree)?;

        // second pass: lower function bodies
        self.lower_functions(tree)?;

        Ok(())
    }

    /// Declare all functions in the module (first pass).
    fn declare_functions(&mut self, tree: &NodeTree) -> Result<(), CraneliftError> {
        for (function_id, function) in tree.iter_nodes::<Function>() {
            let signature = self.create_signature(tree, function)?;
            let name = self.strings.get(function.name);

            let func_id = self
                .module
                .declare_function(name, Linkage::Export, &signature)?;

            self.function_ids.insert(function_id, func_id);
        }

        Ok(())
    }

    /// Lower all function bodies (second pass).
    fn lower_functions(&mut self, tree: &NodeTree) -> Result<(), CraneliftError> {
        for (function_id, function) in tree.iter_nodes::<Function>() {
            let func_id = self.function_ids[&function_id];
            let name = self.strings.get(function.name).to_string();

            let mut context = Context::new();
            context.func.signature = self.create_signature(tree, function)?;
            context.func.name = cranelift_codegen::ir::UserFuncName::user(0, func_id.as_u32());

            // lower the function body
            let lowerer =
                FunctionLowerer::new(tree, self.strings, function, &self.isa, &self.function_ids);
            lowerer.lower(&mut context.func)?;

            // save for CLIF output
            self.compiled_functions.push((name, context.func.clone()));

            // compile and define
            self.module.define_function(func_id, &mut context)?;
        }

        Ok(())
    }

    /// Create a Cranelift signature for a MIR function.
    fn create_signature(
        &self,
        tree: &NodeTree,
        function: &Function,
    ) -> Result<cranelift_codegen::ir::Signature, CraneliftError> {
        use super::types::{is_void_type, lower_type};
        use cranelift_codegen::ir::AbiParam;

        let call_conv = self.isa.default_call_conv();
        let mut signature = cranelift_codegen::ir::Signature::new(call_conv);

        // parameters
        for param in &function.parameters {
            let ty = lower_type(tree, param.ty)?;
            signature.params.push(AbiParam::new(ty));
        }

        // return type (if not void)
        if !is_void_type(tree, function.return_type) {
            let ty = lower_type(tree, function.return_type)?;
            signature.returns.push(AbiParam::new(ty));
        }

        Ok(signature)
    }

    /// Finish lowering and produce the output bytes.
    pub(crate) fn finish(self) -> Result<Vec<u8>, CraneliftError> {
        let product = self.module.finish();
        Ok(product.emit().map_err(|e| CraneliftError::Internal {
            message: e.to_string(),
        })?)
    }

    /// Get the Cranelift IR text format for all functions.
    pub(crate) fn to_clif_string(&self) -> Result<String, CraneliftError> {
        let mut output = String::new();

        for (name, func) in &self.compiled_functions {
            output.push_str(&format!("; function: {name}\n"));
            output.push_str(&func.display().to_string());
            output.push('\n');
        }

        Ok(output)
    }
}

/// Extension trait for pipe operator.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
