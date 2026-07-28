use destack_dir as dir;

use super::types::format_global_type;
use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

macro_rules! formatted {
    ($expression:expr) => {
        match $expression? {
            Some(text) => text,
            None => return Ok(None),
        }
    };
}

/// Formatter for declaration and call signatures in one checked module.
struct SignatureFormatter<'owner, 'module, 'program> {
    /// The checked module being formatted.
    module: &'owner ModuleQueryContext<'module>,
    /// The program used for global type reads.
    program: &'owner ProgramQueryContext<'program>,
}

impl<'owner, 'module, 'program> SignatureFormatter<'owner, 'module, 'program> {
    /// Create a signature formatter for one checked module.
    fn new(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
    ) -> Self {
        Self { module, program }
    }

    /// Format one declaration signature.
    fn declaration_signature(&self, declaration: &dir::Declaration) -> QueryResult<Option<String>> {
        let Some(name) = self.module.declaration_display_name(declaration) else {
            return Ok(None);
        };

        self.declaration_text(declaration, &name)
    }

    /// Format one declaration signature body.
    fn declaration_text(
        &self,
        declaration: &dir::Declaration,
        name: &str,
    ) -> QueryResult<Option<String>> {
        let declaration_prefix = format_declaration_prefix(declaration);

        let text = match declaration {
            dir::Declaration::Function(declaration) => {
                return self.function_text(name, &declaration.signature, &declaration_prefix);
            }
            dir::Declaration::Global(_) => format!("{declaration_prefix}global"),
            dir::Declaration::Module(_) => format!("{declaration_prefix}module"),
            dir::Declaration::Struct(declaration) => {
                let generics_text = formatted!(self.generics(&declaration.generic_parameters));
                format!("{declaration_prefix}struct {name}{generics_text}")
            }
            dir::Declaration::Class(declaration) => {
                let generics_text = formatted!(self.generics(&declaration.generic_parameters));
                format!("{declaration_prefix}class {name}{generics_text}")
            }
            dir::Declaration::Interface(declaration) => {
                let generics_text = formatted!(self.generics(&declaration.generic_parameters));
                let keyword = if declaration.is_nominal {
                    "newtype interface"
                } else {
                    "interface"
                };

                format!("{declaration_prefix}{keyword} {name}{generics_text}")
            }
            dir::Declaration::Enum(_) => {
                format!("{declaration_prefix}enum {name}")
            }
            dir::Declaration::Type(declaration) => {
                let generics_text = formatted!(self.generics(&declaration.generic_parameters));
                let value_id = declaration.value.into_global_any(self.module.module_id());
                let value = formatted!(self.type_text(value_id));
                let keyword = if declaration.is_nominal {
                    "newtype"
                } else {
                    "type"
                };

                format!("{declaration_prefix}{keyword} {name}{generics_text} = {value}")
            }
            dir::Declaration::Extension(_) => {
                format!("{declaration_prefix}extension {name}")
            }
        };

        Ok(Some(text))
    }

    /// Format a function signature with parameters and return type.
    fn function_text(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        declaration_prefix: &str,
    ) -> QueryResult<Option<String>> {
        let phase_prefix = format_function_phase_prefix(signature.phase);

        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };

        let generics_text = formatted!(self.generics(&signature.generic_parameters));
        let parameters_text =
            formatted!(self.parameters(signature.this_parameter, &signature.parameters));
        let return_text = formatted!(self.return_type(signature.return_type));

        Ok(Some(format!(
            "{declaration_prefix}{phase_prefix}{async_prefix}function {name}{generics_text}({parameters_text}){return_text}"
        )))
    }

    /// Format a function or method call signature without declaration keywords.
    fn call_signature(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> QueryResult<Option<String>> {
        let phase_prefix = format_function_phase_prefix(signature.phase);

        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };

        let generics_text = formatted!(self.generics(&signature.generic_parameters));

        let this_parameter = if include_this {
            signature.this_parameter
        } else {
            None
        };

        let parameter_labels =
            formatted!(self.parameter_labels(this_parameter, &signature.parameters));
        let parameters_text = parameter_labels.join(", ");

        let return_text = formatted!(self.return_type(signature.return_type));

        Ok(Some(format!(
            "{phase_prefix}{async_prefix}{name}{generics_text}({parameters_text}){return_text}"
        )))
    }

    /// Format one function signature without a callable name.
    fn function_detail(
        &self,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> QueryResult<Option<String>> {
        self.call_signature("", signature, include_this)
    }

    /// Format one optional return type suffix.
    fn return_type(
        &self,
        return_type: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<Option<String>> {
        let Some(return_node) = return_type else {
            return Ok(Some(String::new()));
        };

        let node_id = return_node.into_global_any(self.module.module_id());
        let type_text = formatted!(self.type_text(node_id));

        Ok(Some(format!(": {type_text}")))
    }

    /// Format generic type parameters.
    fn generics(
        &self,
        generics: &[dir::LocalNodeId<dir::GenericParameter>],
    ) -> QueryResult<Option<String>> {
        if generics.is_empty() {
            return Ok(Some(String::new()));
        }

        let mut formatted = Vec::with_capacity(generics.len());
        for parameter_id in generics {
            formatted.push(formatted!(self.generic_parameter(*parameter_id)));
        }

        Ok(Some(format!("<{}>", formatted.join(", "))))
    }

    /// Format one generic parameter.
    fn generic_parameter(
        &self,
        parameter_id: dir::LocalNodeId<dir::GenericParameter>,
    ) -> QueryResult<Option<String>> {
        let parameter = self
            .module
            .view()
            .get::<dir::GenericParameter>(parameter_id);
        let strings = self.module.strings();

        let text = match parameter {
            dir::GenericParameter::Type {
                name,
                is_const,
                variance,
                ..
            } => {
                let prefix = format_type_generic_parameter_prefix(*is_const, *variance, false);

                format!("{prefix}{}", strings.get(*name))
            }
            dir::GenericParameter::VariadicType {
                name,
                is_const,
                variance,
                ..
            } => {
                let prefix = format_type_generic_parameter_prefix(*is_const, *variance, true);

                format!("{prefix}{}", strings.get(*name))
            }
            dir::GenericParameter::Value {
                name, is_comptime, ..
            } => {
                let prefix = format_value_generic_parameter_prefix(*is_comptime, false);

                format!("{prefix}{}", strings.get(*name))
            }
            dir::GenericParameter::VariadicValue {
                name, is_comptime, ..
            } => {
                let prefix = format_value_generic_parameter_prefix(*is_comptime, true);

                format!("{prefix}{}", strings.get(*name))
            }
            dir::GenericParameter::Lifetime { name } => strings.get(*name).to_string(),
            dir::GenericParameter::Error => return Ok(None),
        };

        Ok(Some(text))
    }

    /// Format function parameters with their types.
    fn parameters(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> QueryResult<Option<String>> {
        let formatted = formatted!(self.parameter_labels(this_parameter, parameters));

        Ok(Some(formatted.join(", ")))
    }

    /// Format parameter labels in declared order.
    fn parameter_labels(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> QueryResult<Option<Vec<String>>> {
        let mut formatted: Vec<String> = Vec::new();

        if let Some(this_parameter) = this_parameter {
            let this_text = formatted!(self.parameter(this_parameter));
            formatted.push(format!("this: {this_text}"));
        }

        for parameter_id in parameters {
            formatted.push(formatted!(self.parameter(*parameter_id)));
        }

        Ok(Some(formatted))
    }

    /// Format one parameter with its type annotation.
    fn parameter(
        &self,
        parameter_id: dir::LocalNodeId<dir::Parameter>,
    ) -> QueryResult<Option<String>> {
        let parameter = self.module.view().get::<dir::Parameter>(parameter_id);
        let Some(name) = self.module.parameter_name(parameter)? else {
            return Ok(None);
        };
        let node_id = parameter_id.into_global_any(self.module.module_id());
        let type_text = formatted!(self.type_text(node_id));

        Ok(Some(format!("{name}: {type_text}")))
    }

    /// Format one checked node type.
    fn type_text(&self, node_id: dir::GlobalNodeIdAny) -> QueryResult<Option<String>> {
        let Some(type_id) = self.module.types().get_node_type_id(node_id) else {
            return Ok(None);
        };

        format_global_type(type_id, self.module, self.program)
    }
}

/// Format the phase prefix for one function.
fn format_function_phase_prefix(phase: dir::FunctionPhase) -> &'static str {
    match phase {
        dir::FunctionPhase::Normal => "",
        dir::FunctionPhase::Comptime => "comptime ",
    }
}

/// Format the declaration prefix keywords for a declaration.
fn format_declaration_prefix(declaration: &dir::Declaration) -> String {
    let export_prefix = match declaration.export() {
        Some(dir::ExportKind::Named) => "export ",
        Some(dir::ExportKind::Default) => "export default ",
        None => "",
    };

    let declare_prefix = if declaration.is_ambient() {
        "declare "
    } else {
        ""
    };

    let abstract_prefix = if declaration.is_abstract() {
        "abstract "
    } else {
        ""
    };

    format!("{export_prefix}{declare_prefix}{abstract_prefix}")
}

/// Format a function or method call signature without declaration keywords.
pub(crate) fn format_call_signature(
    name: &str,
    signature: &dir::FunctionSignature,
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
    include_this: bool,
) -> QueryResult<Option<String>> {
    SignatureFormatter::new(module, program).call_signature(name, signature, include_this)
}

/// Format a function signature without a callable name.
pub(crate) fn format_function_detail(
    signature: &dir::FunctionSignature,
    module: &ModuleQueryContext<'_>,
    program: &ProgramQueryContext<'_>,
    include_this: bool,
) -> QueryResult<Option<String>> {
    SignatureFormatter::new(module, program).function_detail(signature, include_this)
}

/// Format the prefix for one type generic parameter.
fn format_type_generic_parameter_prefix(
    is_const: bool,
    variance: Option<dir::VarianceModifier>,
    is_variadic: bool,
) -> String {
    let mut prefix = String::new();

    if is_const {
        prefix.push_str("const ");
    }

    match variance {
        Some(dir::VarianceModifier::In) => prefix.push_str("in "),
        Some(dir::VarianceModifier::Out) => prefix.push_str("out "),
        Some(dir::VarianceModifier::InOut) => prefix.push_str("in out "),
        None => {}
    }

    if is_variadic {
        prefix.push_str("...");
    }

    prefix
}

/// Format the prefix for one value generic parameter.
fn format_value_generic_parameter_prefix(is_comptime: bool, is_variadic: bool) -> String {
    let mut prefix = String::new();

    if is_comptime {
        prefix.push_str("comptime ");
    }

    if is_variadic {
        prefix.push_str("...");
    }

    prefix
}

/// Format a symbol's signature for hover display.
pub(crate) fn format_symbol_signature(
    program: &ProgramQueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> QueryResult<Option<String>> {
    // resolve owning module
    let module = program.module(symbol_id.module_id)?;
    let declaration_ref = {
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };

        declaration
    };

    // read the declaration from the tree
    let view = module.view();
    let declaration_id = declaration_ref
        .local_id
        .try_into_typed::<dir::Declaration>()
        .map_err(|_| QueryError::invalid(format!("signature symbol: {symbol_id:?}")))?;
    let declaration = view.get::<dir::Declaration>(declaration_id);

    SignatureFormatter::new(module, program).declaration_signature(declaration)
}

/// Format the authored generic parameters for one declaration symbol.
pub(crate) fn format_symbol_generics(
    program: &ProgramQueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> QueryResult<Option<String>> {
    let module = program.module(symbol_id.module_id)?;
    let symbols = module.symbols();
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let source = symbol.declaration.ok_or(QueryError::invalid(format!(
        "type item symbol: {symbol_id:?}"
    )))?;
    if source.local_id.ty != dir::NodeType::Declaration {
        return Err(QueryError::invalid(format!(
            "type item symbol: {symbol_id:?}"
        )));
    }

    // read generic parameters from the exact declaration node
    let declaration_id = source
        .local_id
        .try_into_typed::<dir::Declaration>()
        .map_err(|_| QueryError::invalid(format!("type item symbol: {symbol_id:?}")))?;
    let declaration = module.view().get::<dir::Declaration>(declaration_id);
    let parameters = declaration
        .generic_parameters()
        .ok_or(QueryError::invalid(format!(
            "type item symbol: {symbol_id:?}"
        )))?;
    let formatted = SignatureFormatter::new(module, program)
        .generics(parameters)?
        .ok_or(QueryError::invalid(format!(
            "type item formatting: {symbol_id:?}"
        )))?;

    Ok((!formatted.is_empty()).then_some(formatted))
}
