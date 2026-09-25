use std::error::Error;
use std::fmt;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
pub use tspp_artifact::ConditionSet;
use tspp_serde::Reflect;
use tspp_source::matches;

use crate::{builtin_modes, builtin_roles};

use super::Dependency;

/// Built-in condition aliases accepted in module file suffixes.
const BUILTIN_SUFFIX_ALIASES: &[BuiltinSuffixAlias] = &[
    BuiltinSuffixAlias::new(ConditionAxis::Host, "native"),
    BuiltinSuffixAlias::new(ConditionAxis::Host, "browser"),
    BuiltinSuffixAlias::new(ConditionAxis::Host, "wasi"),
    BuiltinSuffixAlias::new(ConditionAxis::Host, "emscripten"),
    BuiltinSuffixAlias::new(ConditionAxis::Host, "freestanding"),
    BuiltinSuffixAlias::new(ConditionAxis::Runtime, "tspp"),
    BuiltinSuffixAlias::new(ConditionAxis::Runtime, "js"),
];

/// One built-in condition alias accepted in module file suffixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BuiltinSuffixAlias {
    /// The condition axis selected by the alias.
    axis: ConditionAxis,
    /// The alias name accepted in the suffix.
    name: &'static str,
}

impl BuiltinSuffixAlias {
    /// Create one built-in suffix alias.
    const fn new(axis: ConditionAxis, name: &'static str) -> Self {
        Self { axis, name }
    }

    /// Return this alias as a condition gate.
    fn gate(self) -> ConditionGate {
        ConditionGate::axis(self.axis, self.name)
    }

    /// Return the built-in suffix alias with this name.
    fn named(name: &str) -> Option<Self> {
        BUILTIN_SUFFIX_ALIASES
            .iter()
            .find(|alias| alias.name == name)
            .copied()
    }
}

/// Named source graph condition.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    /// Human-readable condition description.
    pub description: Option<String>,
    /// Condition labels.
    pub labels: IndexMap<String, String>,
    /// Condition names included before this condition.
    pub extends: Vec<String>,
    /// Dependencies enabled by this condition.
    pub dependencies: IndexMap<String, Dependency>,
}

/// Named condition declarations from `package.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionCatalog {
    /// Named source graph modes.
    pub modes: IndexMap<String, Condition>,
    /// Named source graph roles.
    pub roles: IndexMap<String, Condition>,
    /// Named optional source graph features.
    pub features: IndexMap<String, Condition>,
    /// Named source graph tags.
    pub tags: IndexMap<String, Condition>,
    /// Named condition aliases.
    pub aliases: IndexMap<String, ConditionRef>,
}

impl ConditionCatalog {
    /// Expand selected condition names through their declared ancestors.
    pub fn expand<'a>(
        &self,
        axis: ConditionAxis,
        selected: impl IntoIterator<Item = &'a str>,
    ) -> Result<IndexSet<String>, ConditionError> {
        let Some(declarations) = self.axis_names(axis) else {
            return Ok(selected.into_iter().map(str::to_string).collect());
        };
        let mut conditions = IndexSet::new();

        // expand each selected condition in ancestor first order
        for selected_name in selected {
            let mut pending = vec![(selected_name, 0)];

            while let Some((name, parent_index)) = pending.last().copied() {
                // skip conditions expanded through another path
                if conditions.contains(name) {
                    pending.pop();
                }
                // expand the next declared parent
                else if let Some(parent) = declarations
                    .get(name)
                    .and_then(|condition| condition.extends.get(parent_index))
                {
                    let index = pending.len() - 1;
                    pending[index].1 += 1;

                    // reject undeclared parents
                    if !declarations.contains_key(parent) {
                        return Err(ConditionError::UnknownParent {
                            axis,
                            condition: name.to_string(),
                            parent: parent.clone(),
                        });
                    }

                    // reject cycles at their first repeated condition
                    if let Some(position) = pending
                        .iter()
                        .position(|(active, _)| *active == parent.as_str())
                    {
                        let mut conditions = pending[position..]
                            .iter()
                            .map(|(active, _)| (*active).to_string())
                            .collect::<Vec<_>>();
                        conditions.push(parent.clone());

                        return Err(ConditionError::InheritanceCycle { axis, conditions });
                    }

                    pending.push((parent, 0));
                }
                // complete conditions without another declared parent
                else {
                    conditions.insert(name.to_string());
                    pending.pop();
                }
            }
        }

        Ok(conditions)
    }

    /// Resolve one condition reference against this catalog.
    pub fn resolve(&self, reference: &ConditionRef) -> Result<ConditionGate, ConditionError> {
        self.resolve_reference(reference, &mut Vec::new())
    }

    /// Resolve one condition reference while tracking aliases.
    fn resolve_reference(
        &self,
        reference: &ConditionRef,
        aliases: &mut Vec<String>,
    ) -> Result<ConditionGate, ConditionError> {
        match reference {
            ConditionRef::Name(name) => self.resolve_name(name, aliases),
            ConditionRef::Predicate(predicate) => self.resolve_predicate(predicate, aliases),
        }
    }

    /// Return every unambiguous condition name accepted in file suffixes.
    pub fn suffix_aliases(&self) -> Result<IndexMap<String, ConditionGate>, ConditionError> {
        // preserve explicit aliases
        let mut aliases = IndexMap::new();
        for (name, reference) in &self.aliases {
            let gate = self.resolve_alias(name, reference, &mut Vec::new())?;

            aliases.insert(name.clone(), gate);
        }

        // add declared source graph names
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Mode)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Role)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Feature)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Tag)?;

        // add built-in suffix aliases through normal name resolution
        for alias in BUILTIN_SUFFIX_ALIASES {
            if aliases.contains_key(alias.name) {
                continue;
            }

            let gate = self.resolve_unprefixed_name(alias.name)?;
            aliases.insert(alias.name.to_string(), gate);
        }

        Ok(aliases)
    }

    /// Resolve one named condition or prefixed condition reference.
    fn resolve_name(
        &self,
        name: &str,
        aliases: &mut Vec<String>,
    ) -> Result<ConditionGate, ConditionError> {
        // resolve explicit axis references
        if let Some((axis, value)) = name.split_once(':') {
            return self.resolve_axis_name(axis, value);
        }

        // resolve explicit aliases
        if let Some(reference) = self.aliases.get(name) {
            return self.resolve_alias(name, reference, aliases);
        }

        // resolve declared condition names
        self.resolve_unprefixed_name(name)
    }

    /// Resolve one named alias while detecting cycles.
    fn resolve_alias(
        &self,
        name: &str,
        reference: &ConditionRef,
        aliases: &mut Vec<String>,
    ) -> Result<ConditionGate, ConditionError> {
        if aliases.iter().any(|alias| alias == name) {
            let mut cycle = aliases.clone();
            cycle.push(name.to_string());

            return Err(ConditionError::AliasCycle { aliases: cycle });
        }

        aliases.push(name.to_string());
        let gate = self.resolve_reference(reference, aliases);
        aliases.pop();

        gate
    }

    /// Resolve one inline condition predicate.
    fn resolve_predicate(
        &self,
        predicate: &ConditionPredicate,
        aliases: &mut Vec<String>,
    ) -> Result<ConditionGate, ConditionError> {
        let all = predicate
            .all
            .as_ref()
            .map(|references| self.resolve_references(references, aliases))
            .transpose()?;
        let any = predicate
            .any
            .as_ref()
            .map(|references| self.resolve_references(references, aliases))
            .transpose()?;
        let not = predicate
            .not
            .as_ref()
            .map(|reference| self.resolve_reference(reference, aliases).map(Box::new))
            .transpose()?;

        Ok(ConditionGate {
            mode: predicate.mode.clone(),
            role: predicate.role.clone(),
            feature: predicate.feature.clone(),
            tag: predicate.tag.clone(),
            target: predicate.target.clone(),
            product: predicate.product.clone(),
            platform: predicate.platform.clone(),
            host: predicate.host.clone(),
            runtime: predicate.runtime.clone(),
            all,
            any,
            not,
        })
    }

    /// Resolve condition references in declaration order.
    fn resolve_references(
        &self,
        references: &[ConditionRef],
        aliases: &mut Vec<String>,
    ) -> Result<Vec<ConditionGate>, ConditionError> {
        references
            .iter()
            .map(|reference| self.resolve_reference(reference, aliases))
            .collect()
    }

    /// Resolve one prefixed condition reference.
    fn resolve_axis_name(&self, axis: &str, name: &str) -> Result<ConditionGate, ConditionError> {
        // parse the axis prefix
        let Some(axis) = ConditionAxis::parse(axis) else {
            return Err(ConditionError::UnknownAxis {
                axis: axis.to_string(),
            });
        };

        // reject empty names
        if name.is_empty() {
            return Err(ConditionError::EmptyName { axis });
        }

        Ok(ConditionGate::axis(axis, name))
    }

    /// Resolve one unprefixed condition name.
    fn resolve_unprefixed_name(&self, name: &str) -> Result<ConditionGate, ConditionError> {
        let mut gate = None;

        // scan source graph axes
        self.resolve_axis_alias(&mut gate, ConditionAxis::Mode, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Role, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Feature, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Tag, name)?;

        if let Some(gate) = gate {
            return Ok(gate);
        }

        if let Some(alias) = BuiltinSuffixAlias::named(name) {
            return Ok(alias.gate());
        }

        Err(ConditionError::UnknownName {
            name: name.to_string(),
        })
    }

    /// Resolve one axis alias candidate.
    fn resolve_axis_alias(
        &self,
        gate: &mut Option<ConditionGate>,
        axis: ConditionAxis,
        name: &str,
    ) -> Result<(), ConditionError> {
        // skip axes without this name
        if !self.contains_axis_name(axis, name) {
            return Ok(());
        }

        // reject ambiguous unprefixed names
        if gate.is_some() {
            return Err(ConditionError::AmbiguousName {
                name: name.to_string(),
            });
        }

        *gate = Some(ConditionGate::axis(axis, name));

        Ok(())
    }

    /// Insert every unambiguous condition name on one axis.
    fn insert_axis_aliases(
        &self,
        aliases: &mut IndexMap<String, ConditionGate>,
        axis: ConditionAxis,
    ) -> Result<(), ConditionError> {
        // skip scalar axes
        let Some(names) = self.axis_names(axis) else {
            return Ok(());
        };

        // add aliases that are not explicitly declared
        for name in names.keys() {
            if aliases.contains_key(name) {
                continue;
            }

            let gate = self.resolve_unprefixed_name(name)?;
            aliases.insert(name.clone(), gate);
        }

        Ok(())
    }

    /// Return whether one axis declares a condition name.
    fn contains_axis_name(&self, axis: ConditionAxis, name: &str) -> bool {
        self.axis_names(axis)
            .is_some_and(|names| names.contains_key(name))
    }

    /// Return the declared source graph names for one axis.
    fn axis_names(&self, axis: ConditionAxis) -> Option<&IndexMap<String, Condition>> {
        match axis {
            ConditionAxis::Mode => Some(&self.modes),
            ConditionAxis::Role => Some(&self.roles),
            ConditionAxis::Feature => Some(&self.features),
            ConditionAxis::Tag => Some(&self.tags),
            ConditionAxis::Target
            | ConditionAxis::Product
            | ConditionAxis::Platform
            | ConditionAxis::Host
            | ConditionAxis::Runtime => None,
        }
    }
}

/// Source-level reference to one active condition predicate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ConditionRef {
    /// Named condition alias or `axis:name` reference.
    Name(String),
    /// Inline condition gate.
    Predicate(Box<ConditionPredicate>),
}

impl Default for ConditionRef {
    fn default() -> Self {
        Self::Predicate(Box::default())
    }
}

/// Error raised while resolving a condition catalog.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionError {
    /// The prefixed condition axis is unknown.
    UnknownAxis { axis: String },
    /// The prefixed condition name is empty.
    EmptyName { axis: ConditionAxis },
    /// The unprefixed condition name is unknown.
    UnknownName { name: String },
    /// The unprefixed condition name exists on multiple axes.
    AmbiguousName { name: String },
    /// The condition aliases recursively include one another.
    AliasCycle { aliases: Vec<String> },
    /// A condition includes an undeclared parent.
    UnknownParent {
        /// The condition axis.
        axis: ConditionAxis,
        /// The child condition name.
        condition: String,
        /// The missing parent condition name.
        parent: String,
    },
    /// Condition inheritance contains a cycle.
    InheritanceCycle {
        /// The condition axis.
        axis: ConditionAxis,
        /// The repeated condition path.
        conditions: Vec<String>,
    },
}

impl fmt::Display for ConditionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAxis { axis } => write!(formatter, "unknown condition axis '{axis}'"),
            Self::EmptyName { axis } => {
                write!(formatter, "empty condition name for '{}' axis", axis.name())
            }
            Self::UnknownName { name } => write!(formatter, "unknown condition '{name}'"),
            Self::AmbiguousName { name } => {
                write!(
                    formatter,
                    "ambiguous condition '{name}', use an explicit axis prefix"
                )
            }
            Self::AliasCycle { aliases } => {
                write!(formatter, "condition alias cycle: {}", aliases.join(" -> "))
            }
            Self::UnknownParent {
                axis,
                condition,
                parent,
            } => write!(
                formatter,
                "{} condition '{condition}' extends unknown condition '{parent}'",
                axis.name()
            ),
            Self::InheritanceCycle { axis, conditions } => write!(
                formatter,
                "{} condition inheritance cycle: {}",
                axis.name(),
                conditions.join(" -> ")
            ),
        }
    }
}

impl Error for ConditionError {}

/// Declared condition predicate before named references are resolved.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionPredicate {
    /// Active mode selector.
    pub mode: Option<ConditionSelector>,
    /// Active role selector.
    pub role: Option<ConditionSelector>,
    /// Active feature selector.
    pub feature: Option<ConditionSelector>,
    /// Active tag selector.
    pub tag: Option<ConditionSelector>,
    /// Active build target selector.
    pub target: Option<ConditionSelector>,
    /// Active product selector.
    pub product: Option<ConditionSelector>,
    /// Active target platform selector.
    pub platform: Option<ConditionSelector>,
    /// Active host environment selector.
    pub host: Option<ConditionSelector>,
    /// Active runtime selector.
    pub runtime: Option<ConditionSelector>,
    /// Predicates that must all match.
    pub all: Option<Vec<ConditionRef>>,
    /// Predicates where at least one must match.
    pub any: Option<Vec<ConditionRef>>,
    /// Predicate that must not match.
    pub not: Option<Box<ConditionRef>>,
}

/// Predicate over active source graph and runtime conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct ConditionGate {
    /// Active mode selector.
    pub mode: Option<ConditionSelector>,
    /// Active role selector.
    pub role: Option<ConditionSelector>,
    /// Active feature selector.
    pub feature: Option<ConditionSelector>,
    /// Active tag selector.
    pub tag: Option<ConditionSelector>,
    /// Active build target selector.
    pub target: Option<ConditionSelector>,
    /// Active product selector.
    pub product: Option<ConditionSelector>,
    /// Active target platform selector.
    pub platform: Option<ConditionSelector>,
    /// Active host environment selector.
    pub host: Option<ConditionSelector>,
    /// Active runtime selector.
    pub runtime: Option<ConditionSelector>,
    /// Predicates that must all match.
    pub all: Option<Vec<ConditionGate>>,
    /// Predicates where at least one must match.
    pub any: Option<Vec<ConditionGate>>,
    /// Predicate that must not match.
    pub not: Option<Box<ConditionGate>>,
}

impl ConditionGate {
    /// Create a gate that matches one active condition on an axis.
    pub fn axis(axis: ConditionAxis, name: impl Into<String>) -> Self {
        match axis {
            ConditionAxis::Mode => Self::mode(name),
            ConditionAxis::Role => Self::role(name),
            ConditionAxis::Feature => Self::feature(name),
            ConditionAxis::Tag => Self::tag(name),
            ConditionAxis::Target => Self {
                target: Some(ConditionSelector::exact(name)),
                ..Self::default()
            },
            ConditionAxis::Product => Self {
                product: Some(ConditionSelector::exact(name)),
                ..Self::default()
            },
            ConditionAxis::Platform => Self {
                platform: Some(ConditionSelector::exact(name)),
                ..Self::default()
            },
            ConditionAxis::Host => Self {
                host: Some(ConditionSelector::exact(name)),
                ..Self::default()
            },
            ConditionAxis::Runtime => Self {
                runtime: Some(ConditionSelector::exact(name)),
                ..Self::default()
            },
        }
    }

    /// Create a gate that matches one active mode.
    pub fn mode(name: impl Into<String>) -> Self {
        Self {
            mode: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active role.
    pub fn role(name: impl Into<String>) -> Self {
        Self {
            role: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active feature.
    pub fn feature(name: impl Into<String>) -> Self {
        Self {
            feature: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active tag.
    pub fn tag(name: impl Into<String>) -> Self {
        Self {
            tag: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Return whether this gate has no selectors.
    pub fn is_empty(&self) -> bool {
        self.mode.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.role.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .feature
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.tag.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.target.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .product
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self
                .platform
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.host.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .runtime
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.all.as_ref().is_none_or(Vec::is_empty)
            && self.any.is_none()
            && self.not.is_none()
    }

    /// Return true when this gate matches one active condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        // source graph sets
        let source_graph_matches = self.matches_set(&self.mode, &conditions.modes)
            && self.matches_set(&self.role, &conditions.roles)
            && self.matches_set(&self.feature, &conditions.features)
            && self.matches_set(&self.tag, &conditions.tags);

        // profile selectors
        let profile_matches = self.matches_scalar(&self.target, conditions.target.as_deref())
            && self.matches_scalar(&self.product, conditions.product.as_deref())
            && self.matches_scalar(&self.platform, Some(conditions.platform.canonical_tag()))
            && self.matches_scalar(&self.host, Some(conditions.host.canonical_tag()))
            && self.matches_scalar(&self.runtime, Some(conditions.runtime.canonical_tag()));

        let all_matches = self
            .all
            .as_ref()
            .is_none_or(|gates| gates.iter().all(|gate| gate.matches(conditions)));
        let any_matches = self
            .any
            .as_ref()
            .is_none_or(|gates| gates.iter().any(|gate| gate.matches(conditions)));
        let not_matches = self
            .not
            .as_ref()
            .is_none_or(|gate| !gate.matches(conditions));

        source_graph_matches && profile_matches && all_matches && any_matches && not_matches
    }

    /// Return true when an optional selector matches one set axis.
    fn matches_set(&self, selector: &Option<ConditionSelector>, names: &IndexSet<String>) -> bool {
        let Some(selector) = selector else {
            return true;
        };

        selector.matches_any(names.iter().map(String::as_str))
    }

    /// Return true when an optional selector matches one scalar axis.
    fn matches_scalar(&self, selector: &Option<ConditionSelector>, name: Option<&str>) -> bool {
        let Some(selector) = selector else {
            return true;
        };
        let Some(name) = name else {
            return false;
        };

        selector.matches(name)
    }
}

/// Return the built-in condition aliases.
pub fn builtin_condition_aliases() -> IndexMap<String, ConditionGate> {
    let mut aliases = IndexMap::new();

    for name in builtin_modes().keys() {
        aliases.insert(name.clone(), ConditionGate::mode(name.clone()));
    }

    for name in builtin_roles().keys() {
        aliases.insert(name.clone(), ConditionGate::role(name.clone()));
    }

    for alias in BUILTIN_SUFFIX_ALIASES {
        aliases.insert(alias.name.to_string(), alias.gate());
    }

    aliases
}

/// One condition axis name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ConditionAxis {
    /// Source graph mode.
    Mode,
    /// Source graph role.
    Role,
    /// Optional source graph feature.
    Feature,
    /// Source graph tag.
    Tag,
    /// Build target.
    Target,
    /// Deliverable product.
    Product,
    /// Target platform.
    Platform,
    /// Host environment.
    Host,
    /// Runtime kind.
    Runtime,
}

impl ConditionAxis {
    /// Parse one condition axis prefix.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "mode" => Some(Self::Mode),
            "role" => Some(Self::Role),
            "feature" => Some(Self::Feature),
            "tag" => Some(Self::Tag),
            "target" => Some(Self::Target),
            "product" => Some(Self::Product),
            "platform" => Some(Self::Platform),
            "host" => Some(Self::Host),
            "runtime" => Some(Self::Runtime),
            _ => None,
        }
    }

    /// Return the canonical condition axis name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::Role => "role",
            Self::Feature => "feature",
            Self::Tag => "tag",
            Self::Target => "target",
            Self::Product => "product",
            Self::Platform => "platform",
            Self::Host => "host",
            Self::Runtime => "runtime",
        }
    }
}

/// Selector over one active condition axis.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConditionSelector {
    /// Condition names or glob patterns.
    pub patterns: Vec<String>,
}

impl<'de> Deserialize<'de> for ConditionSelector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ConditionSelectorValue::deserialize(deserializer).map(Into::into)
    }
}

impl ConditionSelector {
    /// Create an exact name selector.
    pub fn exact(name: impl Into<String>) -> Self {
        Self {
            patterns: vec![name.into()],
        }
    }

    /// Return whether this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Return whether this selector matches one condition name.
    pub fn matches(&self, name: &str) -> bool {
        self.patterns.is_empty()
            || self
                .patterns
                .iter()
                .any(|pattern| glob_match(pattern, name))
    }

    /// Return whether this selector matches any condition name.
    pub fn matches_any<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> bool {
        self.patterns.is_empty() || names.into_iter().any(|name| self.matches(name))
    }
}

/// Deserialized selector shorthand.
#[derive(Deserialize)]
#[serde(untagged)]
enum ConditionSelectorValue {
    /// Exact selector name.
    Exact(String),
    /// Selector patterns.
    Patterns(Vec<String>),
    /// Full selector object.
    Object {
        /// Condition names or glob patterns.
        #[serde(default)]
        patterns: Vec<String>,
    },
}

impl From<ConditionSelectorValue> for ConditionSelector {
    fn from(value: ConditionSelectorValue) -> Self {
        match value {
            ConditionSelectorValue::Exact(pattern) => Self {
                patterns: vec![pattern],
            },
            ConditionSelectorValue::Patterns(patterns)
            | ConditionSelectorValue::Object { patterns } => Self { patterns },
        }
    }
}

/// Return true when one glob-like pattern matches text.
fn glob_match(pattern: &str, text: &str) -> bool {
    matches(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tspp_artifact::{Host, Platform, Runtime};

    use super::*;

    fn condition_ref(json: &str) -> ConditionRef {
        serde_json::from_str(json).unwrap()
    }

    fn conditions(modes: &[&str], roles: &[&str]) -> ConditionSet {
        ConditionSet {
            modes: modes.iter().map(|name| (*name).to_string()).collect(),
            roles: roles.iter().map(|name| (*name).to_string()).collect(),
            features: IndexSet::new(),
            tags: IndexSet::new(),
            target: None,
            product: None,
            role: None,
            labels: BTreeMap::new(),
            platform: Platform::Unknown,
            host: Host::Native,
            runtime: Runtime::Tspp,
        }
    }

    #[test]
    fn test_match_all_and_not_predicates() {
        let catalog = ConditionCatalog::default();
        let gate = catalog
            .resolve(&condition_ref(
                r#"{ "all": ["role:server", { "not": "mode:test" }] }"#,
            ))
            .unwrap();

        assert!(gate.matches(&conditions(&["dev"], &["server"])));
        assert!(!gate.matches(&conditions(&["test"], &["server"])));
        assert!(!gate.matches(&conditions(&["dev"], &["client"])));
    }

    #[test]
    fn test_match_any_predicate() {
        let catalog = ConditionCatalog::default();
        let gate = catalog
            .resolve(&condition_ref(
                r#"{ "any": ["role:client", "role:server"] }"#,
            ))
            .unwrap();

        assert!(gate.matches(&conditions(&[], &["client"])));
        assert!(gate.matches(&conditions(&[], &["server"])));
        assert!(!gate.matches(&conditions(&[], &["worker"])));
    }

    #[test]
    fn test_parse_selector_shorthand() {
        let catalog = ConditionCatalog::default();
        let exact = catalog
            .resolve(&condition_ref(r#"{ "role": "server" }"#))
            .unwrap();
        let either = catalog
            .resolve(&condition_ref(r#"{ "role": ["client", "server"] }"#))
            .unwrap();

        assert!(exact.matches(&conditions(&[], &["server"])));
        assert!(!exact.matches(&conditions(&[], &["client"])));
        assert!(either.matches(&conditions(&[], &["client"])));
        assert!(either.matches(&conditions(&[], &["server"])));
    }

    #[test]
    fn test_suffix_aliases_include_runtime_and_host() {
        let catalog = ConditionCatalog::default();
        let aliases = catalog.suffix_aliases().unwrap();
        let browser = aliases.get("browser").unwrap();
        let js = aliases.get("js").unwrap();

        assert_eq!(
            browser,
            &ConditionGate::axis(ConditionAxis::Host, "browser")
        );
        assert_eq!(js, &ConditionGate::axis(ConditionAxis::Runtime, "js"));
    }

    #[test]
    fn test_suffix_aliases_preserve_declared_names() {
        let mut catalog = ConditionCatalog::default();
        catalog
            .roles
            .insert("browser".to_string(), Condition::default());
        let aliases = catalog.suffix_aliases().unwrap();
        let browser = aliases.get("browser").unwrap();

        assert_eq!(browser, &ConditionGate::role("browser"));
    }

    #[test]
    fn test_builtin_condition_aliases_include_runtime_and_host() {
        let aliases = builtin_condition_aliases();
        let native = aliases.get("native").unwrap();
        let tspp = aliases.get("tspp").unwrap();

        assert_eq!(native, &ConditionGate::axis(ConditionAxis::Host, "native"));
        assert_eq!(tspp, &ConditionGate::axis(ConditionAxis::Runtime, "tspp"));
    }
}
