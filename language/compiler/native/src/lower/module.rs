use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::binemit::CodeOffset;
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::{Context, ir as cir};
use cranelift_module::{DataId, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use destack_core::StringPool;
use destack_mir as mir;

use super::FunctionLowerer;
use super::r#static::lower_static_data;
use super::r#type::lower_type;
use crate::{CodegenCraneliftError, CodegenCraneliftResult};

/// Internal output from module lowering.
#[derive(Debug)]
pub(crate) struct ModuleLowerOutput {
    /// Generated native output (object file or wasm).
    pub bytes: Vec<u8>,
    /// Non-fatal errors encountered during generation.
    pub errors: Vec<CodegenCraneliftError>,
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
    /// Mapping from MIR global ids to Cranelift data ids.
    cl_global_data_ids: HashMap<mir::LocalNodeId<mir::Global>, DataId>,
    /// Compiled Cranelift functions (for CLIF output).
    cl_functions: Vec<(String, cir::Function)>,
    /// Collected non-fatal errors.
    errors: Vec<CodegenCraneliftError>,
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
            cl_global_data_ids: HashMap::new(),
            cl_functions: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Lower an entire MIR module.
    pub(crate) fn lower_module(&mut self, tree: &mir::Tree) -> CodegenCraneliftResult<()> {
        // phase 1: declare all functions
        self.declare_functions(tree)?;

        // phase 2: declare and define all globals
        self.lower_globals(tree)?;

        // phase 3: lower function bodies
        self.lower_functions(tree)?;

        Ok(())
    }

    /// Convert MIR linkage to Cranelift linkage.
    fn convert_linkage(&self, linkage: mir::Linkage) -> Linkage {
        match linkage {
            mir::Linkage::Local => Linkage::Local,
            mir::Linkage::Export => Linkage::Export,
            mir::Linkage::Import => Linkage::Import,
        }
    }

    /// Declare all functions in the module (first pass).
    fn declare_functions(&mut self, tree: &mir::Tree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let signature = self.create_signature(tree, function, pointer_bytes)?;
            let name = self.strings.get(function.name);

            // convert MIR linkage to Cranelift linkage
            let linkage = self.convert_linkage(function.linkage);

            // create cranelift function
            let cl_function_id = self.cl_module.declare_function(name, linkage, &signature)?;
            self.cl_function_ids.insert(function_id, cl_function_id);
        }

        Ok(())
    }

    /// Lower all globals to Cranelift data sections.
    fn lower_globals(&mut self, tree: &mir::Tree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (global_id, global) in tree.iter_nodes::<mir::Global>() {
            let name = self.strings.get(global.name);

            // convert MIR linkage to Cranelift linkage
            let linkage = self.convert_linkage(global.linkage);

            // declare the data section
            let writable = global.is_mutable();
            let data_id = self
                .cl_module
                .declare_data(name, linkage, writable, false)?;

            // define the data if we have an initializer (not for imports)
            if let Some(ref init) = global.initializer {
                let data = lower_static_data(tree, init, global.ty, pointer_bytes)?;
                let mut data_description = cranelift_module::DataDescription::new();
                data_description.define(data.bytes.into_boxed_slice());
                for relocation in data.relocations {
                    let function = self
                        .cl_function_ids
                        .get(&relocation.target)
                        .copied()
                        .ok_or_else(|| CodegenCraneliftError::Internal {
                            message: "missing function declaration for static initializer"
                                .to_string(),
                        })?;
                    let function = self
                        .cl_module
                        .declare_func_in_data(function, &mut data_description);
                    data_description
                        .write_function_addr(relocation.byte_offset as CodeOffset, function);
                }
                self.cl_module.define_data(data_id, &data_description)?;
            }

            self.cl_global_data_ids.insert(global_id, data_id);
        }

        Ok(())
    }

    /// Lower / define all function bodies (third pass after declaration).
    fn lower_functions(&mut self, tree: &mir::Tree) -> CodegenCraneliftResult<()> {
        let pointer_bytes = self.isa.pointer_bytes();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            // skip imported functions (they have no body to define)
            if function.is_import() {
                continue;
            }

            let cl_function_id = self.cl_function_ids[&function_id];
            let name = self.strings.get(function.name).to_string();

            // create cranelift context
            let mut context = Context::new();
            context.func.signature = self.create_signature(tree, function, pointer_bytes)?;
            context.func.name = cir::UserFuncName::user(0, cl_function_id.as_u32());

            // lower the function body
            let function_lowerer = FunctionLowerer::new(
                tree,
                function,
                &self.isa,
                &mut self.cl_module,
                &self.cl_function_ids,
                &self.cl_global_data_ids,
                pointer_bytes,
            );
            function_lowerer.lower(&mut context.func)?;

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
        tree: &mir::Tree,
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

        // function environment parameter when used
        if let Some(environment) = function.environment {
            let ty = lower_type(tree, environment, pointer_bytes)?;
            signature.params.push(cir::AbiParam::new(ty));
        }

        // return type (if not void)
        let return_type = function.return_type;
        if !matches!(tree.get(return_type), mir::Type::Void) {
            let ty = lower_type(tree, return_type, pointer_bytes)?;
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
            errors: self.errors,
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
