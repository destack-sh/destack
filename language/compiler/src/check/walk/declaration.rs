use destack_dir as dir;

use crate::check::{CheckModuleState, Constraint, TypeTerm};

impl CheckModuleState {
    /// Walk one declaration and collect declaration-owned check work.
    pub(in crate::check) fn walk_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        if let dir::Declaration::Function(function) = declaration {
            self.walk_function_declaration(id, function);
        }

        dir::walk_declaration(self, tree, id, declaration);
    }

    /// Walk one declaration member and collect direct declared member types.
    pub(in crate::check) fn walk_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        self.walk_member_type(id, member);
        dir::walk_member(self, tree, id, member);
    }

    /// Walk one type member and collect direct declared member types.
    pub(in crate::check) fn walk_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) {
        self.walk_type_member_type(id, member);
        dir::walk_type_member(self, tree, id, member);
    }

    /// Walk the declared type attached to one declaration member.
    fn walk_member_type(&mut self, id: dir::LocalNodeId<dir::Member>, member: &dir::Member) {
        let ty = match member {
            dir::Member::Field { declared_type, .. }
            | dir::Member::AssociatedConst { declared_type, .. } => *declared_type,
            dir::Member::AssociatedType { value, .. } => *value,
            _ => None,
        };
        let Some(ty) = ty else {
            return;
        };
        let Some(symbol) = self.symbol_for_declaration(id.into_any()) else {
            return;
        };
        let symbol = self.infer_symbol(symbol);
        let ty = self.source_type_infer(ty);

        self.push_constraint(Constraint::Equals {
            left: symbol,
            right: ty,
        });
    }

    /// Walk one function declaration signature.
    fn walk_function_declaration(
        &mut self,
        id: dir::LocalNodeId<dir::Declaration>,
        function: &dir::FunctionDeclaration,
    ) {
        let Some(symbol) = self.symbol_for_declaration(id.into_any()) else {
            return;
        };
        let parameters = function
            .signature
            .parameters
            .iter()
            .filter_map(|parameter| {
                self.parsed()
                    .tree
                    .get(*parameter)
                    .declared_type()
                    .map(|ty| self.source_type_infer(ty))
            })
            .collect();
        let return_type = function
            .signature
            .return_type
            .map(|ty| self.source_type_infer(ty));
        let symbol_type = self.infer_symbol(symbol);

        self.push_constraint(Constraint::Bind {
            result: symbol_type,
            term: TypeTerm::Function {
                source: id.into_global_any(self.module()),
                asynchrony: function.signature.asynchrony,
                parameters,
                return_type,
                is_generator: function.signature.is_generator,
            },
        });
    }

    /// Walk the declared type attached to one interface member.
    fn walk_type_member_type(
        &mut self,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) {
        let ty = match member {
            dir::TypeMember::Field { declared_type, .. }
            | dir::TypeMember::AssociatedConst { declared_type, .. } => *declared_type,
            dir::TypeMember::AssociatedType {
                value, constraint, ..
            } => value.or(*constraint),
            _ => None,
        };
        let Some(ty) = ty else {
            return;
        };
        let Some(symbol) = self.symbol_for_declaration(id.into_any()) else {
            return;
        };
        let symbol = self.infer_symbol(symbol);
        let ty = self.source_type_infer(ty);

        self.push_constraint(Constraint::Equals {
            left: symbol,
            right: ty,
        });
    }
}
