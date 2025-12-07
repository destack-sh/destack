//! Module-level lowering from MIR to Cranelift IR.
//!
//! Module lowering works in two phases:
//!
//! 1. **Declaration phase**: Declare all functions with their signatures.
//!    This allows functions to call each other regardless of definition order.
//!
//! 2. **Definition phase**: Lower each function body using `FunctionLowerer`.
//!    The function id map from phase 1 is used to resolve call targets.

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
use crate::{CodegenCraneliftError, CodegenCraneliftResult, CodegenCraneliftWarning};

/// Internal output from module lowering (bytes + warnings).
#[derive(Debug)]
pub(crate) struct ModuleLowerOutput {
    /// Generated binary output (object file or wasm).
    pub bytes: Vec<u8>,
    /// Warnings encountered during generation.
    pub warnings: Vec<CodegenCraneliftWarning>,
}

/// Module context for lowering a MIR module to Cranelift.
pub(crate) struct ModuleLowerer<'a> {
    /// The target ISA.
    isa: Arc<dyn TargetIsa>,
    /// String pool for interned names.
    strings: &'a StringPool,
    /// The Cranelift object module.
    cl_module: ObjectModule,
    /// Mapping from MIR function ids to Cranelift function ids.
    cl_function_ids: HashMap<mir::LocalNodeId<mir::Function>, FuncId>,
    /// Compiled Cranelift functions (for CLIF output).
    cl_functions: Vec<(String, cir::Function)>,
    /// Collected warnings.
    warnings: Vec<CodegenCraneliftWarning>,
}

impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(isa: Arc<dyn TargetIsa>, strings: &'a StringPool, name: &str) -> Self {
        let builder =
            ObjectBuilder::new(isa.clone(), name, cranelift_module::default_libcall_names())
                .expect("failed to create object builder");
        let module = ObjectModule::new(builder);
        Self {
            isa,
            strings,
            cl_module: module,
            cl_function_ids: HashMap::new(),
            cl_functions: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Record a warning.
    #[allow(dead_code)]
    pub(crate) fn warning(&mut self, warning: CodegenCraneliftWarning) {
        self.warnings.push(warning);
    }

    /// Lower an entire MIR module.
    pub(crate) fn lower_module(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        // phase 1: declare all functions
        self.declare_functions(tree)?;

        // phase 2: lower function bodies
        self.lower_functions(tree)?;

        Ok(())
    }

    /// Declare all functions in the module (first pass).
    fn declare_functions(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let signature = self.create_signature(tree, function, pointer_bytes)?;
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
    fn lower_functions(&mut self, tree: &mir::NodeTree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let cl_function_id = self.cl_function_ids[&function_id];
            let name = self.strings.get(function.name).to_string();

            // create cranelift context
            let mut context = Context::new();
            context.func.signature = self.create_signature(tree, function, pointer_bytes)?;
            context.func.name = cir::UserFuncName::user(0, cl_function_id.as_u32());

            // lower the function body
            let lowerer = FunctionLowerer::new(
                tree,
                self.strings,
                function,
                &self.isa,
                &self.cl_function_ids,
                pointer_bytes,
            );
            lowerer.lower(&mut context.func)?;

            // save for CLIF output
            self.cl_functions.push((name, context.func.clone()));

            // compile and define
            self.cl_module
                .define_function(cl_function_id, &mut context)?;
        }

        Ok(())
    }

    /// Create a Cranelift signature for a MIR function.
    fn create_signature(
        &self,
        tree: &mir::NodeTree,
        function: &mir::Function,
        pointer_bytes: u8,
    ) -> Result<cir::Signature, CodegenCraneliftError> {
        let call_conv = self.isa.default_call_conv();
        let mut signature = cir::Signature::new(call_conv);

        // parameters
        for param in &function.parameters {
            let ty = lower_type(tree, param.ty, pointer_bytes)?;
            signature.params.push(cir::AbiParam::new(ty));
        }

        // return type (if not void)
        if !matches!(tree.get(function.return_type), mir::Type::Void) {
            let ty = lower_type(tree, function.return_type, pointer_bytes)?;
            signature.returns.push(cir::AbiParam::new(ty));
        }

        Ok(signature)
    }

    /// Finish lowering and produce output.
    pub(crate) fn finish(self) -> Result<ModuleLowerOutput, CodegenCraneliftError> {
        let product = self.cl_module.finish();
        let bytes = product
            .emit()
            .map_err(|e| CodegenCraneliftError::Internal {
                message: e.to_string(),
            })?;

        Ok(ModuleLowerOutput {
            bytes,
            warnings: self.warnings,
        })
    }

    /// Get the Cranelift IR text format for all functions.
    pub(crate) fn as_clif_string(&self) -> CodegenCraneliftResult<String> {
        let mut output = String::new();
        for (name, func) in &self.cl_functions {
            output.push_str(&format!("; function: {name}\n"));
            output.push_str(&func.display().to_string());
            output.push('\n');
        }
        Ok(output)
    }
}
