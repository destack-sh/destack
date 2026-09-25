use tspp_core::StringId;

use crate::source::TokenType;
use crate::{
    AttributeArgs, AttributeIdentifier, AttributeKeyValue, AttributeValue, Binding,
    BindingAffinity, BindingEffect, BindingProvider, BindingReplay,
};

use super::{ParseError, ParseResult, Parser};

impl Parser {
    /// Parse the syntax of one runtime binding declaration.
    pub(super) fn parse_binding_args(&mut self) -> ParseResult<AttributeArgs> {
        let name = self.parse_attribute_literal()?;
        self.eat_token(TokenType::Comma)?;
        let options = self.parse_attribute_literal()?;

        Ok(AttributeArgs::Values(vec![name, options]))
    }

    /// Parse one complete runtime binding declaration.
    pub(super) fn parse_binding(&self, args: &AttributeArgs) -> ParseResult<Binding> {
        let AttributeArgs::Values(values) = args else {
            return Err(ParseError::new(
                "binding expects a name and options object",
                self.pos(),
            ));
        };
        let [
            AttributeValue::String(name),
            AttributeValue::Object(options),
        ] = values.as_slice()
        else {
            return Err(ParseError::new(
                "binding expects a string name and options object",
                self.pos(),
            ));
        };

        // reject options that cannot be represented by Binding
        self.ensure_binding_options(options)?;

        // resolve the required semantic properties
        let provider = self
            .binding_string(options, "provider")?
            .ok_or_else(|| ParseError::new("binding provider is required", self.pos()))?;
        let provider = BindingProvider::from_name(self.strings.get(provider))
            .ok_or_else(|| ParseError::new("unknown binding provider", self.pos()))?;
        let effect = self
            .binding_string(options, "effect")?
            .ok_or_else(|| ParseError::new("binding effect is required", self.pos()))?;
        let effect = BindingEffect::from_name(self.strings.get(effect))
            .ok_or_else(|| ParseError::new("unknown binding effect", self.pos()))?;

        // resolve optional execution properties
        let replay = self.binding_replay(options)?;
        let affinity = self.binding_affinity(options)?;
        let is_park = self.binding_boolean(options, "park")?;

        Ok(Binding {
            name: *name,
            provider,
            effect,
            replay,
            affinity,
            is_park,
            requires: self.binding_strings(options, "requires")?,
            platforms: self.binding_strings(options, "platforms")?,
            families: self.binding_strings(options, "families")?,
            hosts: self.binding_strings(options, "hosts")?,
        })
    }

    /// Reject unknown binding options.
    fn ensure_binding_options(&self, options: &[AttributeKeyValue]) -> ParseResult<()> {
        for option in options {
            let AttributeIdentifier::Identifier(name) = option.key else {
                return Err(ParseError::new("binding option is malformed", self.pos()));
            };

            if !matches!(
                self.strings.get(name),
                "provider"
                    | "effect"
                    | "replay"
                    | "affinity"
                    | "park"
                    | "requires"
                    | "platforms"
                    | "families"
                    | "hosts"
            ) {
                return Err(ParseError::new("unknown binding option", self.pos()));
            }
        }

        Ok(())
    }

    /// Return one named binding option exactly once.
    fn binding_option<'a>(
        &self,
        options: &'a [AttributeKeyValue],
        name: &str,
    ) -> ParseResult<Option<&'a AttributeValue>> {
        let mut value = None;

        // retain the one matching option
        for option in options {
            let AttributeIdentifier::Identifier(option_name) = option.key else {
                return Err(ParseError::new("binding option is malformed", self.pos()));
            };
            if self.strings.get(option_name) != name {
                continue;
            }
            if value.replace(&option.value).is_some() {
                return Err(ParseError::new(
                    format!("duplicate binding {name}"),
                    self.pos(),
                ));
            }
        }

        Ok(value)
    }

    /// Return one optional string binding option.
    fn binding_string(
        &self,
        options: &[AttributeKeyValue],
        name: &str,
    ) -> ParseResult<Option<StringId>> {
        let Some(value) = self.binding_option(options, name)? else {
            return Ok(None);
        };
        let AttributeValue::String(value) = value else {
            return Err(ParseError::new(
                format!("binding {name} expects a string"),
                self.pos(),
            ));
        };

        Ok(Some(*value))
    }

    /// Return one optional boolean binding option.
    fn binding_boolean(&self, options: &[AttributeKeyValue], name: &str) -> ParseResult<bool> {
        let Some(value) = self.binding_option(options, name)? else {
            return Ok(false);
        };
        let AttributeValue::Boolean(value) = value else {
            return Err(ParseError::new(
                format!("binding {name} expects a boolean"),
                self.pos(),
            ));
        };

        Ok(*value)
    }

    /// Return one optional string-list binding option.
    fn binding_strings(
        &self,
        options: &[AttributeKeyValue],
        name: &str,
    ) -> ParseResult<Vec<StringId>> {
        let Some(value) = self.binding_option(options, name)? else {
            return Ok(Vec::new());
        };
        let AttributeValue::List(values) = value else {
            return Err(ParseError::new(
                format!("binding {name} expects a string list"),
                self.pos(),
            ));
        };
        let mut strings = Vec::with_capacity(values.len());

        // retain source order exactly
        for value in values {
            let AttributeValue::String(value) = value else {
                return Err(ParseError::new(
                    format!("binding {name} expects a string list"),
                    self.pos(),
                ));
            };
            strings.push(*value);
        }

        Ok(strings)
    }

    /// Return the replay behavior for one binding.
    fn binding_replay(&self, options: &[AttributeKeyValue]) -> ParseResult<BindingReplay> {
        let Some(replay) = self.binding_string(options, "replay")? else {
            return Ok(BindingReplay::Recordable);
        };

        BindingReplay::from_name(self.strings.get(replay))
            .ok_or_else(|| ParseError::new("unknown binding replay policy", self.pos()))
    }

    /// Return the execution affinity for one binding.
    fn binding_affinity(&self, options: &[AttributeKeyValue]) -> ParseResult<BindingAffinity> {
        let Some(affinity) = self.binding_string(options, "affinity")? else {
            return Ok(BindingAffinity::None);
        };

        BindingAffinity::from_name(self.strings.get(affinity))
            .ok_or_else(|| ParseError::new("unknown binding affinity", self.pos()))
    }
}
