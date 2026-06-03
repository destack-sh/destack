use destack_dir as dir;

use crate::check::{Dump, DumpContext, FunctionParameter, FunctionTerm};

use super::format::{dump_list, dump_record};

impl Dump for FunctionTerm {
    /// Render one function type term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        let generic_parameters = self
            .generic_parameters
            .iter()
            .map(|parameter| parameter.dump(context))
            .collect::<Vec<_>>()
            .join(",");
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| parameter.dump(context))
            .collect::<Vec<_>>()
            .join(",");

        dump_record(
            "FunctionTerm",
            [
                ("asynchrony", self.asynchrony.dump(context)),
                ("generics", dump_list(generic_parameters)),
                ("this", self.this_parameter.dump(context)),
                ("parameters", dump_list(parameters)),
                ("return", self.return_type.dump(context)),
                ("generator", self.is_generator.to_string()),
            ],
        )
    }
}

impl Dump for FunctionParameter {
    /// Render one function parameter.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "FunctionParameter",
            [
                ("type", self.ty.dump(context)),
                ("static_parameter", self.static_parameter.dump(context)),
                ("optional", self.is_optional.to_string()),
                ("rest", self.is_rest.to_string()),
            ],
        )
    }
}

impl Dump for dir::FunctionSignature {
    /// Render one source function signature.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "FunctionSignature",
            [
                ("asynchrony", self.asynchrony.dump(context)),
                ("role", self.role.dump(context)),
                ("form", self.form.dump(context)),
                ("phase", self.phase.dump(context)),
                (
                    "generic_parameters",
                    dump_local_node_ids(&self.generic_parameters, context),
                ),
                (
                    "where_clauses",
                    dump_local_node_ids(&self.where_clauses, context),
                ),
                ("this_form", self.this_form.dump(context)),
                ("this_parameter", self.this_parameter.dump(context)),
                ("parameters", dump_local_node_ids(&self.parameters, context)),
                ("return_type", self.return_type.dump(context)),
                ("abstract", self.is_abstract.to_string()),
                ("override", self.is_override.to_string()),
                ("generator", self.is_generator.to_string()),
            ],
        )
    }
}

impl Dump for dir::Asynchrony {
    /// Render one function asynchrony marker.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Sync => "sync".to_string(),
            Self::Async => "async".to_string(),
        }
    }
}

impl Dump for dir::FunctionRole {
    /// Render one function role.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Getter => "getter".to_string(),
            Self::Setter => "setter".to_string(),
            Self::Constructor => "constructor".to_string(),
            Self::New => "new".to_string(),
            Self::Call => "call".to_string(),
        }
    }
}

impl Dump for dir::FunctionForm {
    /// Render one function form.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Function => "function".to_string(),
            Self::Lambda => "lambda".to_string(),
        }
    }
}

impl Dump for dir::FunctionPhase {
    /// Render one function phase.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Normal => "normal".to_string(),
            Self::Comptime => "comptime".to_string(),
        }
    }
}

impl Dump for dir::ThisForm {
    /// Render one receiver source form.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Implicit => "implicit".to_string(),
            Self::Explicit => "explicit".to_string(),
        }
    }
}

impl<T: dir::Node> Dump for dir::LocalNodeId<T> {
    /// Render one local DIR node id.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        format!("local#{}", self.id)
    }
}

/// Render one local node id list.
fn dump_local_node_ids<T: dir::Node>(
    nodes: &[dir::LocalNodeId<T>],
    context: &DumpContext<'_, '_>,
) -> String {
    let nodes = nodes
        .iter()
        .map(|node| node.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(nodes)
}
