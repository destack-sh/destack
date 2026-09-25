use tspp_core::StringId;

use crate::{
    Attribute, AttributeArgs, AttributeIdentifier, AttributeValue, LocalNodeId, Node, Tree, Type,
    TypeDeclaration, TypeId,
};

/// One interface MIR recognizes by its language item key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageItem {
    /// The interface of values a load duplicates.
    Copy,
    /// The interface of values a member call duplicates.
    Clone,
    /// The interface of values a destructor releases.
    Drop,
    /// The interface of numbers with an additive identity.
    Zero,
    /// The interface of numbers with a multiplicative identity.
    One,
}

/// The attribute naming the language item of one declaration.
const ATTRIBUTE: &str = "languageItem";

impl LanguageItem {
    /// Every language item.
    const ALL: [LanguageItem; 5] = [
        LanguageItem::Copy,
        LanguageItem::Clone,
        LanguageItem::Drop,
        LanguageItem::Zero,
        LanguageItem::One,
    ];

    /// Return the stable `@languageItem` key.
    pub fn key(self) -> &'static str {
        match self {
            LanguageItem::Copy => "memory.Copy",
            LanguageItem::Clone => "memory.Clone",
            LanguageItem::Drop => "memory.Drop",
            LanguageItem::Zero => "math.Zero",
            LanguageItem::One => "math.One",
        }
    }

    /// Return the item one key names.
    pub fn from_key(key: StringId) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|item| StringId::for_text(item.key()) == key)
    }

    /// Return the language item key of one declaration.
    pub fn key_of<T>(tree: &Tree, id: LocalNodeId<T>) -> Option<StringId>
    where
        T: Node,
    {
        tree.attributes(id)
            .iter()
            .find_map(|attribute| match (attribute.name, &attribute.args) {
                (
                    AttributeIdentifier::Identifier(name),
                    AttributeArgs::Value(AttributeValue::String(key)),
                ) if name == StringId::for_text(ATTRIBUTE) => Some(*key),
                _ => None,
            })
    }

    /// Return the language item of one declaration.
    pub fn of<T>(tree: &Tree, id: LocalNodeId<T>) -> Option<Self>
    where
        T: Node,
    {
        Self::from_key(Self::key_of(tree, id)?)
    }

    /// Return the language item key on the declaration of one type.
    pub fn key_of_type(tree: &Tree, ty: TypeId) -> Option<StringId> {
        Self::key_of(tree, Self::declaration_of(tree, ty)?)
    }

    /// Return the language item on the declaration of one type.
    pub fn of_type(tree: &Tree, ty: TypeId) -> Option<Self> {
        Self::from_key(Self::key_of_type(tree, ty)?)
    }

    /// Return the attribute recording one language item key on a declaration.
    pub fn attribute(key: StringId) -> Attribute {
        Attribute {
            name: AttributeIdentifier::Identifier(StringId::for_text(ATTRIBUTE)),
            args: AttributeArgs::Value(AttributeValue::String(key)),
        }
    }

    /// Return whether one bound names this item directly or through its heritage.
    pub fn is_bound_by(self, tree: &Tree, bound: TypeId) -> bool {
        self.is_bound_within(tree, bound, &mut Vec::new())
    }

    /// Return whether one bound names this item, skipping the declarations already seen.
    fn is_bound_within(
        self,
        tree: &Tree,
        bound: TypeId,
        seen: &mut Vec<LocalNodeId<TypeDeclaration>>,
    ) -> bool {
        // stop at a bound without a declaration or one already visited
        let Some(declaration) = Self::declaration_of(tree, bound) else {
            return false;
        };
        if seen.contains(&declaration) {
            return false;
        }

        // match this item on the declaration itself
        seen.push(declaration);
        if Self::of(tree, declaration) == Some(self) {
            return true;
        }

        tree.get(declaration)
            .heritage
            .extends
            .iter()
            .any(|parent| self.is_bound_within(tree, *parent, seen))
    }

    /// Return the declaration of one type, reading an application through its base.
    fn declaration_of(tree: &Tree, ty: TypeId) -> Option<LocalNodeId<TypeDeclaration>> {
        let base = match tree.get(ty) {
            Type::Application { base, .. } => *base,
            _ => ty,
        };

        tree.type_declaration(base)
    }
}
