use destack_dir as dir;

use super::types::format_global_type;
use crate::{ModuleQueryContext, declaration_display_name};

/// Formatted declaration signature.
#[derive(Debug, Clone)]
pub struct FormattedSignature {
    /// The full signature string.
    pub text: String,
    /// The declaration kind (function, struct, etc.).
    pub kind: &'static str,
}

/// Formatted call signature for functions and methods.
#[derive(Debug, Clone)]
pub struct FormattedCallSignature {
    /// The full signature label.
    pub label: String,
    /// Parameter labels in declared order.
    pub parameters: Vec<String>,
}

/// Formatter for declaration and call signatures in one checked module.
struct SignatureFormatter<'module, 'repo> {
    /// The checked module being formatted.
    module: &'module ModuleQueryContext<'repo>,
}

/// The signature position that owns one checked type.
#[derive(Debug, Clone, Copy)]
enum SignatureTypeRole {
    /// A function return type.
    Return,
    /// A function parameter type.
    Parameter,
}

impl SignatureTypeRole {
    /// Return the diagnostic label for this type role.
    fn label(self) -> &'static str {
        match self {
            Self::Return => "return",
            Self::Parameter => "parameter",
        }
    }
}

impl<'module, 'repo> SignatureFormatter<'module, 'repo> {
    /// Create a signature formatter for one checked module.
    fn new(module: &'module ModuleQueryContext<'repo>) -> Self {
        Self { module }
    }

    /// Format one declaration signature.
    fn declaration_signature(&self, declaration: &dir::Declaration) -> Option<FormattedSignature> {
        let kind = declaration.kind_name();
        let name = declaration_display_name(self.module.strings(), declaration)?;
        let text = self.declaration_text(declaration, &name);

        Some(FormattedSignature { text, kind })
    }

    /// Format one declaration signature body.
    fn declaration_text(&self, declaration: &dir::Declaration, name: &str) -> String {
        let declaration_prefix = format_declaration_prefix(declaration);

        match declaration {
            dir::Declaration::Function(declaration) => {
                self.function_text(name, &declaration.signature, &declaration_prefix)
            }
            dir::Declaration::Global(_) => format!("{declaration_prefix}global"),
            dir::Declaration::Module(_) => format!("{declaration_prefix}module"),
            dir::Declaration::Struct(declaration) => {
                let generics_text = self.generics(&declaration.generic_parameters);
                format!("{declaration_prefix}struct {name}{generics_text}")
            }
            dir::Declaration::Class(declaration) => {
                let generics_text = self.generics(&declaration.generic_parameters);
                format!("{declaration_prefix}class {name}{generics_text}")
            }
            dir::Declaration::Interface(declaration) => {
                let generics_text = self.generics(&declaration.generic_parameters);
                format!("{declaration_prefix}interface {name}{generics_text}")
            }
            dir::Declaration::Enum(_) => {
                format!("{declaration_prefix}enum {name}")
            }
            dir::Declaration::Type(_) => {
                format!("{declaration_prefix}type {name}")
            }
            dir::Declaration::Extension(_) => {
                format!("{declaration_prefix}extension {name}")
            }
        }
    }

    /// Format a function signature with parameters and return type.
    fn function_text(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        declaration_prefix: &str,
    ) -> String {
        let phase_prefix = format_function_phase_prefix(signature.phase);

        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };

        let generics_text = self.generics(&signature.generic_parameters);
        let parameters_text = self.parameters(signature.this_parameter, &signature.parameters);
        let return_text = self.return_type(signature.return_type);

        format!(
            "{declaration_prefix}{phase_prefix}{async_prefix}function {name}{generics_text}({parameters_text}){return_text}"
        )
    }

    /// Format a function or method call signature without declaration keywords.
    fn call_signature(
        &self,
        name: &str,
        signature: &dir::FunctionSignature,
        include_this: bool,
    ) -> FormattedCallSignature {
        let phase_prefix = format_function_phase_prefix(signature.phase);

        let async_prefix = match signature.asynchrony {
            dir::Asynchrony::Async => "async ",
            dir::Asynchrony::Sync => "",
        };

        let generics_text = self.generics(&signature.generic_parameters);

        let this_parameter = if include_this {
            signature.this_parameter
        } else {
            None
        };

        let parameter_labels = self.parameter_labels(this_parameter, &signature.parameters);
        let parameters_text = parameter_labels.join(", ");

        let return_text = self.return_type(signature.return_type);

        let label = format!(
            "{phase_prefix}{async_prefix}{name}{generics_text}({parameters_text}){return_text}"
        );

        FormattedCallSignature {
            label,
            parameters: parameter_labels,
        }
    }

    /// Format one optional return type suffix.
    fn return_type(&self, return_type: Option<dir::LocalNodeId<dir::TypeExpression>>) -> String {
        let Some(return_node) = return_type else {
            return String::new();
        };

        let node_id = return_node.into_global_any(self.module.module_id());
        let type_text = self.type_text(node_id, SignatureTypeRole::Return);

        format!(": {type_text}")
    }

    /// Format generic type parameters.
    fn generics(&self, generics: &[dir::LocalNodeId<dir::GenericParameter>]) -> String {
        if generics.is_empty() {
            return String::new();
        }

        let formatted: Vec<_> = generics
            .iter()
            .map(|parameter_id| self.generic_parameter(*parameter_id))
            .collect();

        format!("<{}>", formatted.join(", "))
    }

    /// Format one generic parameter.
    fn generic_parameter(&self, parameter_id: dir::LocalNodeId<dir::GenericParameter>) -> String {
        let parameter = self
            .module
            .view()
            .get::<dir::GenericParameter>(parameter_id);
        let strings = self.module.strings();

        match parameter {
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
            dir::GenericParameter::Error => {
                panic!("error generic parameter reached signature formatting")
            }
        }
    }

    /// Format function parameters with their types.
    fn parameters(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> String {
        let formatted = self.parameter_labels(this_parameter, parameters);

        formatted.join(", ")
    }

    /// Format parameter labels in declared order.
    fn parameter_labels(
        &self,
        this_parameter: Option<dir::LocalNodeId<dir::Parameter>>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> Vec<String> {
        let mut formatted: Vec<String> = Vec::new();

        if let Some(this_parameter) = this_parameter {
            let this_text = self.parameter(this_parameter);
            formatted.push(format!("this: {this_text}"));
        }

        formatted.extend(
            parameters
                .iter()
                .map(|parameter_id| self.parameter(*parameter_id)),
        );

        formatted
    }

    /// Format one parameter with its type annotation.
    fn parameter(&self, parameter_id: dir::LocalNodeId<dir::Parameter>) -> String {
        let parameter = self.module.view().get::<dir::Parameter>(parameter_id);
        let name = self.parameter_name(parameter);
        let node_id = parameter_id.into_global_any(self.module.module_id());
        let type_text = self.type_text(node_id, SignatureTypeRole::Parameter);

        format!("{name}: {type_text}")
    }

    /// Format one parameter label.
    fn parameter_name(&self, parameter: &dir::Parameter) -> String {
        let strings = self.module.strings();
        match parameter {
            dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
            dir::Parameter::Pattern { .. } => "_".to_string(),
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = strings.get(*name);
                format!("...{name}")
            }
            dir::Parameter::VariadicPattern { .. } => "..._".to_string(),
            dir::Parameter::Error => panic!("error parameter reached signature formatting"),
        }
    }

    /// Format one checked node type.
    fn type_text(&self, node_id: dir::GlobalNodeIdAny, role: SignatureTypeRole) -> String {
        let role = role.label();
        let type_id = self
            .module
            .types()
            .get_node_type_id(node_id)
            .unwrap_or_else(|| panic!("missing checked {role} type for node {node_id:?}"));

        format_global_type(type_id, self.module)
            .unwrap_or_else(|| panic!("unable to format checked {role} type {type_id:?}"))
    }
}

/// Format a declaration's signature with full type information.
pub fn format_declaration_signature(
    declaration: &dir::Declaration,
    module: &ModuleQueryContext<'_>,
) -> Option<FormattedSignature> {
    SignatureFormatter::new(module).declaration_signature(declaration)
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
    let export_prefix = if declaration.export().is_some() {
        "export "
    } else {
        ""
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
pub fn format_call_signature(
    name: &str,
    signature: &dir::FunctionSignature,
    module: &ModuleQueryContext<'_>,
    include_this: bool,
) -> FormattedCallSignature {
    SignatureFormatter::new(module).call_signature(name, signature, include_this)
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
pub fn format_symbol_signature(
    root_module: &ModuleQueryContext<'_>,
    symbol_id: dir::GlobalSymbolId,
) -> Option<FormattedSignature> {
    // resolve owning module
    let module = root_module.module_context(symbol_id.module_id);
    let declaration_ref = {
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        symbol.declaration?
    };

    // read the declaration from the tree
    let view = module.view();
    let declaration_id = declaration_ref.local_id.try_into().unwrap_or_else(|_| {
        panic!(
            "signature declaration has incompatible node id: {:?}",
            declaration_ref.local_id
        )
    });
    let declaration = view.get::<dir::Declaration>(declaration_id);

    SignatureFormatter::new(&module).declaration_signature(declaration)
}
