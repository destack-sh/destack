use crate::{Copy, LocalNodeId, Symbol, Tree, Type, TypeDeclaration, TypeHeritage, TypeId};

impl Tree {
    /// Reserve one declaration for a recursive or opaque type.
    pub fn reserve_type(&mut self, symbol: Symbol) -> LocalNodeId<TypeDeclaration> {
        // reuse the declaration with this persistent identity
        if let Some(declaration) = self.declared_types.get(&symbol) {
            return *declaration;
        }

        // allocate the declaration before interning its type
        let declaration = TypeDeclaration {
            symbol,
            name: None,
            definition: None,
            generics: Vec::new(),
            heritage: TypeHeritage::default(),
        };
        let local_id = self.type_declarations.allocate(declaration);
        let declaration = self.insert_node(local_id);
        self.intern_type(Type::Declaration { declaration }, Copy::No);
        self.declared_types.insert(symbol, declaration);

        declaration
    }

    /// Return the type declared under one symbol.
    pub fn identified_type(&self, symbol: Symbol) -> Option<TypeId> {
        let declaration = *self.declared_types.get(&symbol)?;

        self.find_type(&Type::Declaration { declaration }, Copy::No)
    }

    /// Return whether a declaration awaits its definition.
    pub fn type_is_reserved(&self, id: TypeId) -> bool {
        self.type_declaration(id)
            .is_some_and(|declaration| self.get(declaration).definition.is_none())
    }

    /// Return the declaration of a type.
    pub fn type_declaration(&self, ty: TypeId) -> Option<LocalNodeId<TypeDeclaration>> {
        match self.get(ty) {
            Type::Declaration { declaration } => Some(*declaration),
            _ => None,
        }
    }

    /// Return the direct heritage of a declared type.
    pub fn type_heritage(&self, ty: TypeId) -> Option<&TypeHeritage> {
        let declaration = self.type_declaration(ty)?;

        Some(&self.get(declaration).heritage)
    }

    /// Return whether a declared type has a definition.
    pub fn is_defined_type(&self, id: TypeId) -> bool {
        self.type_declaration(id)
            .is_some_and(|declaration| self.get(declaration).definition.is_some())
    }

    /// Return whether a type has a declared identity.
    pub fn is_identified_type(&self, id: TypeId) -> bool {
        self.type_declaration(id).is_some()
    }

    /// Return the persistent symbol of a declared type.
    pub fn type_symbol(&self, id: TypeId) -> Option<Symbol> {
        let declaration = self.type_declaration(id)?;

        Some(self.get(declaration).symbol)
    }
}

impl TypeDeclaration {
    /// Return whether this declaration takes generic parameters.
    pub fn is_template(&self) -> bool {
        !self.generics.is_empty()
    }
}
