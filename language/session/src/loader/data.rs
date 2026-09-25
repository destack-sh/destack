use std::sync::Arc;
use tspp_artifact::{ArtifactDependencySet, ArtifactPayload, Data};
use tspp_source::{File, FileType, ModuleId, Span};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
    /// Collect the source closure for one data artifact.
    pub(crate) fn collect_data(
        &self,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactDependencySet, SessionError> {
        let revision = attempt.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleNotTracked { module_id })?;
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_source(revision, module.file_id, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Provide one data artifact through the selected loader.
    pub(crate) fn provide_data(
        &self,
        module_id: ModuleId,
        attempt: &ProviderAttempt,
    ) -> Result<ArtifactPayload, SessionError> {
        let revision = attempt.revision();
        let module = self
            .repository()
            .module(revision, module_id)?
            .ok_or(SessionError::ModuleNotTracked { module_id })?;
        let file = self.source_file(revision, module.file_id)?;
        let data = match file.ty {
            FileType::Json => {
                let value = Self::parse_json_value(file.as_ref())?;
                Data::Json(value.into())
            }
            FileType::Toml => {
                let value = Self::parse_toml_value(file.as_ref())?;
                Data::Json(value.into())
            }
            FileType::Yaml => {
                let value = Self::parse_yaml_value(file.as_ref())?;
                Data::Json(value.into())
            }
            file_type => {
                return Err(SessionError::Internal {
                    detail: format!("module has no parsed data payload: {file_type:?}"),
                });
            }
        };

        Ok(ArtifactPayload::Data(Arc::new(data)))
    }

    /// Parse JSON content into a JSON value.
    fn parse_json_value(file: &File) -> Result<serde_json::Value, SessionError> {
        serde_json::from_str(file.text()).map_err(|error| {
            let line = error.line().checked_sub(1);
            let line = line.and_then(|line| u32::try_from(line).ok());
            let column = error.column().checked_sub(1);
            let column = column.and_then(|column| u32::try_from(column).ok());
            let offset = line
                .zip(column)
                .and_then(|(line, column)| file.get_byte_position(line, column));
            let Some(offset) = offset else {
                return SessionError::Internal {
                    detail: format!(
                        "json parser reported invalid location {}:{}: {error}",
                        error.line(),
                        error.column(),
                    ),
                };
            };
            let span = Span::at(file.id, offset, 1);

            SessionError::Internal {
                detail: format!("json data parse failed at {span:?}: {error}"),
            }
        })
    }

    /// Parse YAML content into a JSON value.
    fn parse_yaml_value(file: &File) -> Result<serde_json::Value, SessionError> {
        serde_yaml_ng::from_str(file.text()).map_err(|error| {
            let span = match error.location() {
                Some(location) => {
                    let offset = u32::try_from(location.index()).ok();
                    let offset = offset.filter(|offset| *offset <= file.len);
                    let Some(offset) = offset else {
                        return SessionError::Internal {
                            detail: format!(
                                "yaml parser reported invalid byte index {}: {error}",
                                location.index(),
                            ),
                        };
                    };

                    Span::at(file.id, offset, 1)
                }
                None => Span::empty(file.id),
            };

            SessionError::Internal {
                detail: format!("yaml data parse failed at {span:?}: {error}"),
            }
        })
    }

    /// Parse TOML content into a JSON value.
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_toml_value(file: &File) -> Result<serde_json::Value, SessionError> {
        let value: toml::Value = toml::from_str(file.text()).map_err(|error| {
            let span = error
                .span()
                .map(|range| Span::new(file.id, range.start as u32, range.end as u32))
                .unwrap_or_else(|| Span::empty(file.id));

            SessionError::Internal {
                detail: format!("toml data parse failed at {span:?}: {}", error.message()),
            }
        })?;

        Self::toml_to_json(value)
    }

    /// Parse TOML content into a JSON value on wasm.
    #[cfg(target_arch = "wasm32")]
    fn parse_toml_value(file: &File) -> Result<serde_json::Value, SessionError> {
        Err(SessionError::Internal {
            detail: format!(
                "toml data parse failed at {:?}: toml imports are not supported on wasm",
                Span::empty(file.id)
            ),
        })
    }

    /// Convert a TOML value to a JSON value.
    #[cfg(not(target_arch = "wasm32"))]
    fn toml_to_json(toml: toml::Value) -> Result<serde_json::Value, SessionError> {
        match toml {
            toml::Value::String(string) => Ok(serde_json::Value::String(string)),
            toml::Value::Integer(integer) => Ok(serde_json::Value::Number(integer.into())),
            toml::Value::Float(float) => {
                let number =
                    serde_json::Number::from_f64(float).ok_or_else(|| SessionError::Internal {
                        detail: format!("toml data parse produced non-json float: {float}"),
                    })?;

                Ok(serde_json::Value::Number(number))
            }
            toml::Value::Boolean(boolean) => Ok(serde_json::Value::Bool(boolean)),
            toml::Value::Datetime(datetime) => Ok(serde_json::Value::String(datetime.to_string())),
            toml::Value::Array(array) => {
                let array = array
                    .into_iter()
                    .map(Self::toml_to_json)
                    .collect::<Result<_, _>>()?;

                Ok(serde_json::Value::Array(array))
            }
            toml::Value::Table(table) => {
                let table = table
                    .into_iter()
                    .map(|(key, value)| Ok((key, Self::toml_to_json(value)?)))
                    .collect::<Result<_, SessionError>>()?;

                Ok(serde_json::Value::Object(table))
            }
        }
    }
}
