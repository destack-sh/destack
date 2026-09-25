use tspp_core::StringId;
use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the declaration one binding decorator names.
    pub(in crate::lower) fn decorator_binding(
        &mut self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<mir::Binding> {
        // read the name and options the decorator holds
        let arguments = self.decorator_arguments(application)?.to_vec();
        let [name, options] = arguments.as_slice() else {
            return Err(CompilerError::Internal {
                message: "a binding decorator without a name and options".to_string(),
            });
        };
        let name = self.binding_string(name, "name")?;
        let Some(properties) = options.as_object() else {
            return Err(CompilerError::Internal {
                message: "a non-object binding options value".to_string(),
            });
        };

        // start every option at its declared default
        let mut effect = None;
        let mut provider = None;
        let mut replay = mir::BindingReplay::Recordable;
        let mut affinity = mir::BindingAffinity::None;
        let mut is_park = false;
        let mut requires = None;
        let mut platforms = Vec::new();
        let mut families = Vec::new();
        let mut hosts = Vec::new();

        // project each option exactly once
        for property in properties {
            // read the option name
            let Some((key, value)) = property.as_field() else {
                return Err(CompilerError::Internal {
                    message: "a non-field property in the binding options".to_string(),
                });
            };
            let Some(key) = key.name() else {
                return Err(CompilerError::Internal {
                    message: "a non-name key in the binding options".to_string(),
                });
            };
            let key = self.strings.get(key);

            // skip omitted optional properties, which evaluate to undefined
            if matches!(value.as_scalar(), Some(dir::Literal::Undefined)) {
                continue;
            }

            // project each known field into its MIR representation
            match key {
                "effect" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    let value = mir::BindingEffect::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("an unknown binding {key} '{value}'"),
                        }
                    })?;
                    effect = Some(value);
                }
                "provider" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    let value = mir::BindingProvider::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("an unknown binding {key} '{value}'"),
                        }
                    })?;
                    provider = Some(value);
                }
                "replay" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    replay = mir::BindingReplay::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("an unknown binding {key} '{value}'"),
                        }
                    })?;
                }
                "affinity" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    affinity = mir::BindingAffinity::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("an unknown binding {key} '{value}'"),
                        }
                    })?;
                }
                "park" => {
                    let Some(dir::Literal::Boolean(value)) = value.as_scalar() else {
                        return Err(CompilerError::Internal {
                            message: format!("a non-boolean binding '{key}'"),
                        });
                    };
                    is_park = value;
                }
                "requires" => requires = Some(self.binding_strings(value, key)?),
                "platforms" => platforms = self.binding_strings(value, key)?,
                "families" => families = self.binding_strings(value, key)?,
                "hosts" => hosts = self.binding_strings(value, key)?,
                _ => {
                    return Err(CompilerError::Internal {
                        message: format!("an unknown binding option '{key}'"),
                    });
                }
            }
        }

        // require the mandatory declaration fields
        let effect = effect.ok_or_else(|| CompilerError::Internal {
            message: "a binding without an 'effect'".to_string(),
        })?;
        let provider = provider.ok_or_else(|| CompilerError::Internal {
            message: "a binding without a 'provider'".to_string(),
        })?;
        let requires = requires.ok_or_else(|| CompilerError::Internal {
            message: "a binding without a 'requires'".to_string(),
        })?;

        // assemble the declaration from its projected options
        let mut binding = mir::Binding::new(name, provider, effect);
        binding.replay = replay;
        binding.affinity = affinity;
        binding.is_park = is_park;
        binding.requires = requires;
        binding.platforms = platforms;
        binding.families = families;
        binding.hosts = hosts;

        Ok(binding)
    }

    /// Return one string from a binding option.
    fn binding_string(&self, value: &dir::StaticTerm, name: &str) -> CompilerResult<StringId> {
        value.as_string().ok_or_else(|| CompilerError::Internal {
            message: format!("a non-string binding '{name}'"),
        })
    }

    /// Return one string array from a binding option.
    fn binding_strings(
        &self,
        value: &dir::StaticTerm,
        name: &str,
    ) -> CompilerResult<Vec<StringId>> {
        let Some(values) = value.as_array() else {
            return Err(CompilerError::Internal {
                message: format!("a non-array binding '{name}'"),
            });
        };

        // convert each element, preserving the declared order
        let mut strings = Vec::with_capacity(values.len());
        for value in values {
            strings.push(self.binding_string(value, name)?);
        }

        Ok(strings)
    }
}
