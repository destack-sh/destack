use tspp_artifact::DiagnosticBuilder;
use tspp_core::{FxIndexSet, StringPool};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckError, CheckState, DecoratorApplication, DecoratorObject};
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
    pub(in crate::sema) fn apply_representation_decorator(
        &mut self,
        module: ModuleId,
        application: &DecoratorApplication,
        value: &dir::StaticTerm,
    ) -> CompilerResult<()> {
        // decode the decorator value and find the declaration it owns
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

        // reject a second representation decorator on the same declaration
        let previous = self
            .module(module)
            .decorators_tail
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
        // reject representations the declaration cannot carry
        let definition = self
            .definition(symbol)?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("representation owner {symbol:?} has no definition"),
            })?;
        let kind = representation_value.representation.kind;
        let has_virtual_dispatch = matches!(
            &*definition,
            dir::Definition::Class(_)
                if kind == dir::RepresentationKind::C
                    && self.class_requires_virtual_dispatch(symbol)?
        );
        if !definition.is_representable(kind) || has_virtual_dispatch {
            let representation = self.strings().get(representation_value.name).to_string();
            self.report_unsupported_representation(module, source.local_id, representation);

            return Ok(());
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

            // read whether the class declares virtual methods, and its base
            let (declares_virtual_dispatch, base) = match self.definition(current)?.as_deref() {
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

            // step to the base class
            symbol = match base {
                Some(base) => {
                    let (_, base) = self.nominal_application(base)?;

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
        // unwrap the decorator's newtype and its argument tuple
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

        // read the written name and options out of the argument list
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
            None => strings.intern("tspp"),
        };

        // read the representation kind the name selects
        let name_text = strings.get(name);
        let kind =
            dir::RepresentationKind::try_from(name_text).map_err(|_| CompilerError::Internal {
                message: format!("representation decorator has invalid name '{name_text}'"),
            })?;
        // build the representation and apply its written options
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
