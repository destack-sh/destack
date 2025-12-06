//! Module-level lowering from MIR to Cranelift IR.
//!
//! Module lowering works in two passes:
//!
//! 1. **Declaration pass**: Declare all functions with their signatures.
//!    This allows functions to call each other regardless of definition order.
//!
//! 2. **Definition pass**: Lower each function body using `FunctionLowerer`.
//!    The function id map from pass 1 is used to resolve call targets.

use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{Context, ir as cir};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use destack_mir as mir;
use destack_source::StringPool;

use super::FunctionLowerer;
use super::r#type::lower_type;
use crate::CraneliftError;

/// Lowers an entire MIR module to Cranelift.
///
/// Manages the Cranelift module, function declarations, and lowering state.
pub(crate) struct ModuleLowerer<'a> {
    /// The target ISA.
    isa: Arc<dyn TargetIsa>,
    /// String pool for resolving names.
    strings: &'a StringPool,
    /// The Cranelift object module.
    cl_module: ObjectModule,
    /// Mapping from MIR function ids to Cranelift function ids.
    cl_function_ids: HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
    /// Compiled Cranelift functions (for CLIF output).
    cl_functions: Vec<(String, cir::Function)>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(isa: &'a dyn TargetIsa, strings: &'a StringPool) -> Self {
        // we need to clone the isa for ObjectBuilder (it takes Arc)
        // TODO avoid looking up the isa again
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
            cl_module: module,
            cl_function_ids: HashMap::new(),
            cl_functions: Vec::new(),
        }
    }

    /// Lower an entire MIR module.
    pub(crate) fn lower_module(&mut self, tree: &mir::NodeTree) -> Result<(), CraneliftError> {
        // first pass: declare all functions
        self.declare_functions(tree)?;

        // second pass: lower function bodies
        self.lower_functions(tree)?;

        Ok(())
    }

    /// Declare all functions in the module (first pass).
    fn declare_functions(&mut self, tree: &mir::NodeTree) -> Result<(), CraneliftError> {
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let signature = self.create_signature(tree, function)?;
            let name = self.strings.get(function.name);

            // create cranelift function
            let cl_function_id =
                self.cl_module
                    .declare_function(&name, Linkage::Export, &signature)?;
            self.cl_function_ids.insert(function_id, cl_function_id);
        }

        Ok(())
    }

    /// Lower / define all function bodies (second pass after declaration).
    fn lower_functions(&mut self, tree: &mir::NodeTree) -> Result<(), CraneliftError> {
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let cl_function_id = self.cl_function_ids[&function_id];
            let name = self.strings.get(function.name).to_string();

            // create cranelift context
            let mut context = Context::new();
            context.func.signature = self.create_signature(tree, function)?;
            context.func.name = cir::UserFuncName::user(0, cl_function_id.as_u32());

            // lower the function body
            let lowerer =
                FunctionLowerer::new(tree, self.strings, function, &self.isa, &self.cl_function_ids);
            lowerer.lower(&mut context.func)?;

            // save for CLIF output
            self.cl_functions.push((name, context.func.clone()));

            // compile and define
            self.cl_module.define_function(cl_function_id, &mut context)?;
        }

        Ok(())
    }

    /// Create a Cranelift signature for a MIR function.
    fn create_signature(
        &self,
        tree: &mir::NodeTree,
        function: &mir::Function,
    ) -> Result<cir::Signature, CraneliftError> {
        let call_conv = self.isa.default_call_conv();
        let mut signature = cir::Signature::new(call_conv);

        // parameters
        for param in &function.parameters {
            let ty = lower_type(tree, param.ty)?;
            signature.params.push(cir::AbiParam::new(ty));
        }

        // return type (if not void)
        if !matches!(tree.get(function.return_type), mir::Type::Void) {
            let ty = lower_type(tree, function.return_type)?;
            signature.returns.push(cir::AbiParam::new(ty));
        }

        Ok(signature)
    }

    /// Finish lowering and produce the output bytes.
    pub(crate) fn finish(self) -> Result<Vec<u8>, CraneliftError> {
        let product = self.cl_module.finish();
        product.emit().map_err(|e| CraneliftError::Internal {
            message: e.to_string(),
        })
    }

    /// Get the Cranelift IR text format for all functions.
    pub(crate) fn as_clif_string(&self) -> Result<String, CraneliftError> {
        let mut output = String::new();
        for (name, func) in &self.cl_functions {
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
