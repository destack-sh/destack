use crate::{
    Copy, GenericArgument, GenericParameter, LanguageItem, ParameterDomain, Tree, Type, TypeId,
};

impl Copy {
    /// Decide whether one type copies under the generic parameters of its declaration.
    pub fn decide(tree: &Tree, ty: TypeId, generics: &[GenericParameter]) -> Copy {
        Self::under(tree, ty, &|index| Self::of_parameter(tree, generics, index))
    }

    /// Decide whether one generic type parameter copies by its bounds.
    pub fn of_parameter(tree: &Tree, generics: &[GenericParameter], index: u32) -> Copy {
        let Some(GenericParameter {
            domain: ParameterDomain::Type { bounds },
            ..
        }) = generics.get(index as usize)
        else {
            return Copy::No;
        };

        match bounds
            .iter()
            .any(|bound| LanguageItem::Copy.is_bound_by(tree, *bound))
        {
            true => Copy::Yes,
            false => Copy::No,
        }
    }

    /// Decide whether one type copies under a decision for each parameter it mentions.
    pub fn under(tree: &Tree, ty: TypeId, parameter: &dyn Fn(u32) -> Copy) -> Copy {
        match tree.get(ty) {
            // decide an application through its base
            Type::Application {
                base, arguments, ..
            } => Self::under(tree, *base, &|index| match arguments.get(index as usize) {
                Some(GenericArgument::Type(argument)) => Self::under(tree, *argument, parameter),
                _ => Copy::Yes,
            }),
            _ if !ty.mentions_parameter(tree) => tree.get(ty).copy(tree),
            Type::Parameter { index } => parameter(*index),
            Type::ManuallyDrop { value } => Self::under(tree, *value, parameter),
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                Self::under(tree, *element, parameter)
            }
            Type::Tuple { elements } => elements.iter().fold(Copy::Yes, |copy, element| {
                copy.combine(Self::under(tree, *element, parameter))
            }),
            Type::Struct { fields, copy } => fields.iter().fold(*copy, |copy, field| {
                copy.combine(Self::under(tree, tree.get(*field).ty, parameter))
            }),
            Type::Newtype { inner, copy } => copy.combine(Self::under(tree, *inner, parameter)),
            Type::Variant { cases, copy, .. } => cases.iter().fold(*copy, |copy, case| {
                copy.combine(Self::under(tree, case.ty, parameter))
            }),
            other => other.copy(tree),
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringId;

    use crate::{
        Copy, Field, GenericParameter, LanguageItem, ParameterDomain, Symbol, Tree, Type,
        TypeHeritage, TypeId,
    };

    /// Declare one empty interface type with an optional language item key.
    fn declare_interface(
        tree: &mut Tree,
        name: &str,
        key: Option<&str>,
        extends: Vec<TypeId>,
    ) -> TypeId {
        let name = StringId::for_text(name);
        let ty = tree.reserve_type(Symbol::named(name));
        tree.define_type(
            ty,
            Type::Struct {
                fields: Vec::new(),
                copy: Copy::Yes,
            },
        );
        let declaration = tree.insert_type_declaration(
            name,
            Vec::new(),
            Vec::new(),
            ty,
            TypeHeritage {
                extends,
                implements: Vec::new(),
            },
        );
        if let Some(key) = key {
            tree.push_attribute(
                declaration,
                LanguageItem::attribute(StringId::for_text(key)),
            );
        }

        ty
    }

    /// A parameter copies when a bound names Copy directly or through heritage.
    #[test]
    fn test_decide_parameter_copy_by_its_bounds() {
        let mut tree = Tree::new();
        let copy = declare_interface(
            &mut tree,
            "Copy",
            Some(LanguageItem::Copy.key()),
            Vec::new(),
        );
        let integer = declare_interface(&mut tree, "Integer", Some("math.Integer"), vec![copy]);
        let marker = declare_interface(&mut tree, "Marker", None, Vec::new());
        let generics = vec![
            GenericParameter {
                name: StringId::for_text("T"),
                domain: ParameterDomain::Type { bounds: vec![copy] },
            },
            GenericParameter {
                name: StringId::for_text("U"),
                domain: ParameterDomain::Type {
                    bounds: vec![integer],
                },
            },
            GenericParameter {
                name: StringId::for_text("V"),
                domain: ParameterDomain::Type {
                    bounds: vec![marker],
                },
            },
        ];

        assert_eq!(Copy::of_parameter(&tree, &generics, 0), Copy::Yes);
        assert_eq!(Copy::of_parameter(&tree, &generics, 1), Copy::Yes);
        assert_eq!(Copy::of_parameter(&tree, &generics, 2), Copy::No);
        assert_eq!(Copy::of_parameter(&tree, &generics, 3), Copy::No);
    }

    /// A template struct copies under a copying parameter and moves under a moving one.
    #[test]
    fn test_decide_template_struct_copy_under_its_parameters() {
        let mut tree = Tree::new();
        let copy = declare_interface(
            &mut tree,
            "Copy",
            Some(LanguageItem::Copy.key()),
            Vec::new(),
        );
        let parameter = tree.intern_type(Type::Parameter { index: 0 });
        let field = tree.intern_field(
            Field {
                name: Some(StringId::for_text("value")),
                ty: parameter,
            },
            Vec::new(),
        );
        let template = tree.intern_type(Type::Struct {
            fields: vec![field],
            copy: Copy::Yes,
        });
        let copying = vec![GenericParameter {
            name: StringId::for_text("T"),
            domain: ParameterDomain::Type { bounds: vec![copy] },
        }];
        let moving = vec![GenericParameter {
            name: StringId::for_text("T"),
            domain: ParameterDomain::Type { bounds: Vec::new() },
        }];

        assert_eq!(
            Copy::decide(&tree, TypeId::from(template), &copying),
            Copy::Yes
        );
        assert_eq!(
            Copy::decide(&tree, TypeId::from(template), &moving),
            Copy::No
        );
    }
}
