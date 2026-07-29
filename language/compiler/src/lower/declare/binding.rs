use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Return the checked declaration carried by one binding decorator.
    pub(in crate::lower) fn decorator_binding(
        &self,
        application: &dir::DecoratorApplication,
    ) -> CompilerResult<mir::Binding> {
        let arguments = self.decorator_arguments(application)?;
        let [name, options] = arguments else {
            return Err(CompilerError::Internal {
                message: "checked binding does not contain name and options".to_string(),
            });
        };
        let name = self.binding_string(name, "name")?;
        let Some(properties) = options.as_object() else {
            return Err(CompilerError::Internal {
                message: "checked binding options are not an object".to_string(),
            });
        };

        // project each checked option exactly once
        let mut effect = None;
        let mut provider = None;
        let mut replay = mir::BindingReplay::Recordable;
        let mut affinity = mir::BindingAffinity::None;
        let mut requires = None;
        let mut platforms = Vec::new();
        let mut families = Vec::new();
        let mut hosts = Vec::new();
        for property in properties {
            let Some((key, value)) = property.as_field() else {
                return Err(CompilerError::Internal {
                    message: "checked binding options contain a non-field property".to_string(),
                });
            };
            let Some(key) = key.name() else {
                return Err(CompilerError::Internal {
                    message: "checked binding option has a non-name key".to_string(),
                });
            };
            let key = self.strings.get(key);

            // omitted optional properties evaluate to undefined
            if matches!(value.as_scalar(), Some(dir::ScalarLiteral::Undefined)) {
                continue;
            }

            // project each known field into its MIR representation
            match key {
                "effect" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    let value = mir::BindingEffect::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("checked binding has unknown {key} '{value}'"),
                        }
                    })?;
                    effect = Some(value);
                }
                "provider" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    let value = mir::BindingProvider::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("checked binding has unknown {key} '{value}'"),
                        }
                    })?;
                    provider = Some(value);
                }
                "replay" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    replay = mir::BindingReplay::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("checked binding has unknown {key} '{value}'"),
                        }
                    })?;
                }
                "affinity" => {
                    let value = self.binding_string(value, key)?;
                    let value = self.strings.get(value);
                    affinity = mir::BindingAffinity::from_name(value).ok_or_else(|| {
                        CompilerError::Internal {
                            message: format!("checked binding has unknown {key} '{value}'"),
                        }
                    })?;
                }
                "requires" => requires = Some(self.binding_strings(value, key)?),
                "platforms" => platforms = self.binding_strings(value, key)?,
                "families" => families = self.binding_strings(value, key)?,
                "hosts" => hosts = self.binding_strings(value, key)?,
                _ => {
                    return Err(CompilerError::Internal {
                        message: format!("checked binding has unknown option '{key}'"),
                    });
                }
            }
        }

        // require the non-optional declaration fields
        let effect = effect.ok_or_else(|| CompilerError::Internal {
            message: "checked binding is missing 'effect'".to_string(),
        })?;
        let provider = provider.ok_or_else(|| CompilerError::Internal {
            message: "checked binding is missing 'provider'".to_string(),
        })?;
        let requires = requires.ok_or_else(|| CompilerError::Internal {
            message: "checked binding is missing 'requires'".to_string(),
        })?;
        let mut binding = mir::Binding::new(name, provider, effect);
        binding.replay = replay;
        binding.affinity = affinity;
        binding.requires = requires;
        binding.platforms = platforms;
        binding.families = families;
        binding.hosts = hosts;

        Ok(binding)
    }

    /// Return one string from a checked binding option.
    fn binding_string(&self, value: &dir::StaticTerm, name: &str) -> CompilerResult<StringId> {
        value.as_string().ok_or_else(|| CompilerError::Internal {
            message: format!("checked binding '{name}' is not a string"),
        })
    }

    /// Return one string array from a checked binding option.
    fn binding_strings(
        &self,
        value: &dir::StaticTerm,
        name: &str,
    ) -> CompilerResult<Vec<StringId>> {
        let Some(values) = value.as_array() else {
            return Err(CompilerError::Internal {
                message: format!("checked binding '{name}' is not an array"),
            });
        };
        let mut strings = Vec::with_capacity(values.len());

        // preserve the declared policy order
        for value in values {
            strings.push(self.binding_string(value, name)?);
        }

        Ok(strings)
    }
}
