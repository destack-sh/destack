use destack_artifact::DiagnosticBuilder;
use destack_core::{FxIndexSet, StringPool};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckError, CheckState, DecoratorApplication, DecoratorObject};
use crate::{CompilerError, CompilerResult};

/// One decoded representation decorator value.
struct RepresentationValue {
    /// The canonical representation.
    representation: dir::Representation,
    /// The authored representation family name.
    name: dir::StringId,
}

impl CheckState<'_> {
    /// Apply one representation decorator to its declaration.
    pub(in crate::check) fn apply_representation_decorator(
        &mut self,
        module: ModuleId,
        application: &DecoratorApplication,
        value: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        let decorator = application.expression.decorator.into_global(module);
        let source = decorator.into_any();
        let representation_value = RepresentationValue::decode(value, self.strings())?;
        let symbol = self
            .module(module)
            .declaration_symbol(application.owner.local_id);
        let Some(symbol) = symbol else {
            let representation = self.strings().get(representation_value.name).to_string();
            self.report_unsupported_representation(module, source.local_id, representation);

            return Ok(());
        };
        let previous = self
            .module(module)
            .decorators
            .iter_applications()
            .map(|(_, application)| application)
            .find(|previous| {
                previous.owner == application.owner
                    && matches!(
                        previous.resolution.target,
                        dir::DecoratorTarget::LanguageItem {
                            item: dir::LanguageItem::ReprDecorator,
                            ..
                        }
                    )
            })
            .map(|application| application.source);
        if let Some(previous) = previous {
            let anchor = self.diagnostic_anchor(module, source.local_id);
            let error = CheckError::DuplicateRepresentationDecorator { anchor, module };
            let previous = self.diagnostic_anchor(module, previous.local_id.into_any());
            let diagnostic =
                DiagnosticBuilder::new(error).label(previous, "first representation selected here");
            self.report(module, diagnostic);

            return Ok(());
        }
        let definition =
            self.definition(symbol)?
                .cloned()
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("representation owner {symbol:?} has no definition"),
                })?;
        let kind = representation_value.representation.kind;
        let has_virtual_dispatch = matches!(
            &definition,
            dir::Definition::Class(_)
                if kind == dir::RepresentationKind::C
                    && self.class_requires_virtual_dispatch(symbol)?
        );
        if !definition.supports_representation(kind) || has_virtual_dispatch {
            let representation = self.strings().get(representation_value.name).to_string();
            self.report_unsupported_representation(module, source.local_id, representation);

            return Ok(());
        }

        // validate enum domains against C and explicit integer representations
        let mut enum_backing = None;
        if let dir::Definition::Enum(definition) = &definition {
            match representation_value.representation.kind {
                dir::RepresentationKind::C
                    if matches!(definition.backing, dir::EnumBackingType::String) =>
                {
                    let anchor = self.diagnostic_anchor(module, source.local_id);
                    self.report(module, CheckError::NonIntegerCEnum { anchor, module });

                    return Ok(());
                }
                dir::RepresentationKind::Integer(integer) => {
                    let backing = dir::EnumBackingType::Integer(integer);
                    let outside = definition.members.iter().find_map(|member| match member {
                        dir::DefinitionMember::EnumVariant(variant)
                            if !backing.contains(variant.value) =>
                        {
                            Some(variant.value)
                        }
                        _ => None,
                    });
                    if let Some(value) = outside {
                        let value = dir::ScalarLiteral::from(value);
                        let value = self.format_scalar_literal(&value);
                        let representation =
                            self.strings().get(representation_value.name).to_string();
                        let anchor = self.diagnostic_anchor(module, source.local_id);
                        let error = CheckError::EnumValueOutsideRepresentation {
                            anchor,
                            module,
                            value,
                            representation,
                        };
                        self.report(module, error);

                        return Ok(());
                    }
                    enum_backing = Some(backing);
                }
                dir::RepresentationKind::Destack
                | dir::RepresentationKind::C
                | dir::RepresentationKind::Transparent => {}
            }
        }

        // commit the layout policy and any explicit enum scalar backing
        let definition = self
            .definition_mut(symbol)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("representation owner {symbol:?} lost its definition"),
            })?;
        match definition {
            dir::Definition::Struct(definition) => {
                definition.representation = representation_value.representation;
            }
            dir::Definition::Class(definition) => {
                definition.representation = representation_value.representation;
            }
            dir::Definition::Enum(definition) => {
                definition.representation = representation_value.representation;
                if let Some(backing) = enum_backing {
                    definition.backing = backing;
                }
            }
            dir::Definition::Newtype(definition) => {
                definition.representation = representation_value.representation;
            }
            dir::Definition::TypeAlias(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Extension(_) => {
                return Err(CompilerError::Internal {
                    message: format!("representation owner {symbol:?} changed definition kind"),
                });
            }
        }

        Ok(())
    }

    /// Return whether a class or one of its bases declares virtual dispatch.
    fn class_requires_virtual_dispatch(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let mut visited = FxIndexSet::default();
        let mut symbol = Some(symbol);

        // follow the single class heritage chain
        while let Some(current) = symbol {
            // stop on cyclic heritage, which has no finite C layout
            if !visited.insert(current) {
                return Ok(true);
            }
            let (declares_virtual_dispatch, base) = match self.definition(current)? {
                Some(dir::Definition::Class(definition)) => (
                    definition.declares_virtual_dispatch(),
                    definition.extends.as_ref().map(|heritage| heritage.ty),
                ),
                _ => {
                    return Err(CompilerError::Internal {
                        message: format!("class {current:?} has no definition"),
                    });
                }
            };

            // stop at the first class declaring virtual methods
            if declares_virtual_dispatch {
                return Ok(true);
            }
            symbol = match base {
                Some(base) => {
                    let (_, base) = self.require_nominal_application(base)?;

                    Some(base.symbol)
                }
                None => None,
            };
        }

        Ok(false)
    }

    /// Report a representation unsupported by its decorated declaration kind.
    fn report_unsupported_representation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        representation: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let error = CheckError::UnsupportedRepresentation {
            anchor,
            module,
            representation,
        };
        self.report(module, error);
    }
}

impl RepresentationValue {
    /// Decode one representation decorator value.
    fn decode(value: &dir::StaticTerm, strings: &StringPool) -> CompilerResult<Self> {
        let Some((_, value)) = value.as_newtype() else {
            return Err(CompilerError::Internal {
                message: "representation decorator has a non-newtype value".to_string(),
            });
        };
        let Some(elements) = value.as_tuple() else {
            return Err(CompilerError::Internal {
                message: "representation decorator has a non-tuple value".to_string(),
            });
        };
        let (name, options) = match elements {
            [value] => {
                if let Some(name) = value.as_string() {
                    (Some(name), None)
                } else if matches!(value, dir::StaticTerm::Object { .. }) {
                    (None, Some(value))
                } else {
                    return Err(CompilerError::Internal {
                        message: "representation decorator has an invalid argument".to_string(),
                    });
                }
            }
            [name, options] => {
                let name = name.as_string().ok_or_else(|| CompilerError::Internal {
                    message: "representation decorator has a non-string name".to_string(),
                })?;

                (Some(name), Some(options))
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: format!("representation decorator has {} arguments", elements.len()),
                });
            }
        };
        // select native layout for the options-only constructor
        let name = match name {
            Some(name) => name,
            None => strings.intern("destack"),
        };
        let name_text = strings.get(name);
        let kind =
            dir::RepresentationKind::try_from(name_text).map_err(|_| CompilerError::Internal {
                message: format!("representation decorator has invalid name '{name_text}'"),
            })?;
        let mut value = Self {
            representation: dir::Representation {
                kind,
                alignment: None,
                packing: None,
            },
            name,
        };
        if let Some(options) = options {
            value.apply_options(options, strings)?;
        }

        Ok(value)
    }

    /// Apply representation options in object evaluation order.
    fn apply_options(
        &mut self,
        value: &dir::StaticTerm,
        strings: &StringPool,
    ) -> CompilerResult<()> {
        let object = DecoratorObject::try_from(value)?;
        for (key, value) in object.fields {
            match strings.get(key) {
                "align" => {
                    let alignment = value.as_integer().ok_or_else(|| CompilerError::Internal {
                        message: "representation alignment has a non-integer value".to_string(),
                    })?;
                    let alignment = u64::try_from(alignment)
                        .ok()
                        .filter(|alignment| alignment.is_power_of_two())
                        .ok_or_else(|| CompilerError::Internal {
                            message: "representation alignment escaped its declared values"
                                .to_string(),
                        })?;
                    self.representation.alignment = Some(alignment);
                }
                "packed" => {
                    let packing = if let Some(is_packed) = value.as_boolean() {
                        is_packed.then_some(1)
                    } else {
                        let packing =
                            value.as_integer().ok_or_else(|| CompilerError::Internal {
                                message: "representation packing has an invalid scalar value"
                                    .to_string(),
                            })?;
                        let packing = u64::try_from(packing)
                            .ok()
                            .filter(|packing| packing.is_power_of_two())
                            .ok_or_else(|| CompilerError::Internal {
                                message: "representation packing escaped its declared values"
                                    .to_string(),
                            })?;

                        Some(packing)
                    };
                    self.representation.packing = packing;
                }
                name => {
                    return Err(CompilerError::Internal {
                        message: format!("representation options contain unknown field '{name}'"),
                    });
                }
            }
        }

        Ok(())
    }
}
