use destack_artifact::{ArtifactPayload, Data};
use destack_source::{File, FileId, FileType, ModuleId, Span};

use crate::{ProviderAttempt, SessionError, SessionState};

impl SessionState {
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
        let file = self.source_file(revision, module.file_id, attempt)?;
        let data = match file.ty {
            FileType::Json => {
                let value = Self::parse_json_value(file.as_ref())?;
                Data::Json(value)
            }
            FileType::Toml => {
                let value = Self::parse_toml_value(file.id, file.text())?;
                Data::Json(value)
            }
            FileType::Yaml => {
                let value = Self::parse_yaml_value(file.id, file.text())?;
                Data::Json(value)
            }
            file_type => {
                return Err(SessionError::Internal {
                    detail: format!("module has no parsed data payload: {file_type:?}"),
                });
            }
        };

        Ok(ArtifactPayload::Data(data))
    }

    /// Parse JSON content into a JSON value.
    fn parse_json_value(file: &File) -> Result<serde_json::Value, SessionError> {
        serde_json::from_str(file.text()).map_err(|error| {
            let offset = File::byte_offset_from_position(file.text(), error.line(), error.column());
            let span = Span::at(file.id, offset, 1);

            SessionError::Internal {
                detail: format!("json data parse failed at {span:?}: {error}"),
            }
        })
    }

    /// Parse YAML content into a JSON value.
    fn parse_yaml_value(file_id: FileId, content: &str) -> Result<serde_json::Value, SessionError> {
        serde_yaml_ng::from_str(content).map_err(|error| {
            let span = error
                .location()
                .map(|location| {
                    let offset = File::byte_offset_from_position(
                        content,
                        location.line(),
                        location.column(),
                    );

                    Span::at(file_id, offset, 1)
                })
                .unwrap_or_else(|| Span::empty(file_id));

            SessionError::Internal {
                detail: format!("yaml data parse failed at {span:?}: {error}"),
            }
        })
    }

    /// Parse TOML content into a JSON value.
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_toml_value(file_id: FileId, content: &str) -> Result<serde_json::Value, SessionError> {
        let value: toml::Value = toml::from_str(content).map_err(|error| {
            let span = error
                .span()
                .map(|range| Span::new(file_id, range.start as u32, range.end as u32))
                .unwrap_or_else(|| Span::empty(file_id));

            SessionError::Internal {
                detail: format!("toml data parse failed at {span:?}: {}", error.message()),
            }
        })?;

        Self::toml_to_json(value)
    }

    /// Parse TOML content into a JSON value on wasm.
    #[cfg(target_arch = "wasm32")]
    fn parse_toml_value(
        file_id: FileId,
        _content: &str,
    ) -> Result<serde_json::Value, SessionError> {
        Err(SessionError::Internal {
            detail: format!(
                "toml data parse failed at {:?}: toml imports are not supported on wasm",
                Span::empty(file_id)
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
