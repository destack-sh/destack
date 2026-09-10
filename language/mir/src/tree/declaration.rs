use destack_core::StringId;

use crate::{
    GenericParameter, LocalNodeId, Symbol, Tree, Type, TypeDeclaration, TypeHeritage, TypeId,
};

impl Tree {
    /// Record the substituted definition of one type application.
    pub fn define_application(&mut self, application: TypeId, definition: TypeId) {
        self.representations.insert(application, definition);
    }

    /// Return the recorded definition of one type application.
    pub fn representation(&self, ty: TypeId) -> Option<TypeId> {
        self.representations.get(&ty).copied()
    }

    /// Return the recorded definition, or the type itself.
    pub fn represented(&self, ty: TypeId) -> TypeId {
        self.representation(ty).unwrap_or(ty)
    }

    /// Reserve one declaration for a recursive or opaque type.
    pub fn reserve_type(&mut self, symbol: Symbol) -> TypeId {
        // reuse the declaration with this persistent identity
        if let Some(id) = self.identified_type(symbol) {
            return id;
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
        let id = self.intern_type(Type::Declaration { declaration });
        self.declared_types.insert(symbol, id);

        id
    }

    /// Return the type declared under one symbol.
    pub fn identified_type(&self, symbol: Symbol) -> Option<TypeId> {
        self.declared_types.get(&symbol).copied()
    }

    /// Return whether a declaration awaits its definition.
    pub fn type_is_reserved(&self, id: TypeId) -> bool {
        self.type_declaration(id)
            .is_some_and(|declaration| self.get(declaration).definition.is_none())
    }

    /// Define one reserved type exactly once.
    pub fn define_type(&mut self, id: TypeId, ty: Type) {
        // require an incomplete declaration
        let Type::Declaration { declaration } = *self.get(id) else {
            panic!("defined structural MIR type {id:?}");
        };
        assert!(
            self.get(declaration).definition.is_none(),
            "defined MIR type {id:?} twice"
        );

        // intern the definition without changing the declared identity
        let definition = self.intern_type(ty);
        let local_id = self.node_local_id(declaration.id);
        self.type_declarations.get_mut(local_id).definition = Some(definition);
    }

    /// Attach source metadata to a reserved declaration.
    pub fn insert_type_declaration(
        &mut self,
        name: StringId,
        generics: Vec<GenericParameter>,
        ty: TypeId,
        heritage: TypeHeritage,
    ) -> LocalNodeId<TypeDeclaration> {
        let Type::Declaration { declaration } = *self.get(ty) else {
            panic!("declared structural MIR type {ty:?}");
        };
        let local_id = self.node_local_id(declaration.id);
        let declared = self.type_declarations.get_mut(local_id);
        assert!(declared.name.is_none(), "declared MIR type {ty:?} twice");

        // complete the source declaration
        declared.name = Some(name);
        declared.generics = generics;
        declared.heritage = heritage;

        declaration
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
