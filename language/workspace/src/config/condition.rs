pub use destack_artifact::ConditionSet;
use destack_source::matches as glob_matches;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use super::Dependency;

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

/// Named condition declarations from `destack.json`.
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
    pub aliases: IndexMap<String, ConditionGate>,
}

impl ConditionCatalog {
    /// Resolve one condition reference against this catalog.
    pub fn resolve(&self, reference: &ConditionRef) -> Result<ConditionGate, ConditionRefError> {
        match reference {
            ConditionRef::Name(name) => self.resolve_name(name),
            ConditionRef::Gate(gate) => Ok(gate.clone()),
        }
    }

    /// Return every unambiguous condition name accepted in file suffixes.
    pub fn suffix_aliases(&self) -> Result<IndexMap<String, ConditionGate>, ConditionRefError> {
        // preserve explicit aliases
        let mut aliases = self.aliases.clone();

        // add declared source graph names
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Mode)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Role)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Feature)?;
        self.insert_axis_aliases(&mut aliases, ConditionAxis::Tag)?;

        Ok(aliases)
    }

    /// Resolve one named condition or prefixed condition reference.
    fn resolve_name(&self, name: &str) -> Result<ConditionGate, ConditionRefError> {
        // resolve explicit axis references
        if let Some((axis, value)) = name.split_once(':') {
            return self.resolve_axis_name(axis, value);
        }

        // resolve explicit aliases
        if let Some(gate) = self.aliases.get(name) {
            return Ok(gate.clone());
        }

        // resolve declared condition names
        self.resolve_unprefixed_name(name)
    }

    /// Resolve one prefixed condition reference.
    fn resolve_axis_name(
        &self,
        axis: &str,
        name: &str,
    ) -> Result<ConditionGate, ConditionRefError> {
        // parse the axis prefix
        let Some(axis) = ConditionAxis::parse(axis) else {
            return Err(ConditionRefError::UnknownAxis {
                axis: axis.to_string(),
            });
        };

        // reject empty names
        if name.is_empty() {
            return Err(ConditionRefError::EmptyName { axis });
        }

        Ok(ConditionGate::axis(axis, name))
    }

    /// Resolve one unprefixed condition name.
    fn resolve_unprefixed_name(&self, name: &str) -> Result<ConditionGate, ConditionRefError> {
        let mut gate = None;

        // scan source graph axes
        self.resolve_axis_alias(&mut gate, ConditionAxis::Mode, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Role, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Feature, name)?;
        self.resolve_axis_alias(&mut gate, ConditionAxis::Tag, name)?;

        gate.ok_or_else(|| ConditionRefError::UnknownName {
            name: name.to_string(),
        })
    }

    /// Resolve one axis alias candidate.
    fn resolve_axis_alias(
        &self,
        gate: &mut Option<ConditionGate>,
        axis: ConditionAxis,
        name: &str,
    ) -> Result<(), ConditionRefError> {
        // skip axes without this name
        if !self.contains_axis_name(axis, name) {
            return Ok(());
        }

        // reject ambiguous unprefixed names
        if gate.is_some() {
            return Err(ConditionRefError::AmbiguousName {
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
    ) -> Result<(), ConditionRefError> {
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
    Gate(ConditionGate),
}

impl Default for ConditionRef {
    fn default() -> Self {
        Self::Gate(ConditionGate::default())
    }
}

/// Error raised while resolving one condition reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConditionRefError {
    /// The prefixed condition axis is unknown.
    UnknownAxis { axis: String },
    /// The prefixed condition name is empty.
    EmptyName { axis: ConditionAxis },
    /// The unprefixed condition name is unknown.
    UnknownName { name: String },
    /// The unprefixed condition name exists on multiple axes.
    AmbiguousName { name: String },
}

impl std::fmt::Display for ConditionRefError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        }
    }
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
    }

    /// Return true when this gate matches one active condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        // source graph sets
        let source_graph_matches = self.matches_set(&self.mode, &conditions.modes)
            && self.matches_set(&self.role, &conditions.roles)
            && self.matches_set(&self.feature, &conditions.features)
            && self.matches_set(&self.tag, &conditions.tags);

        // profile selectors
        let profile_matches = self.matches_optional(&self.target, conditions.target.as_deref())
            && self.matches_optional(&self.product, conditions.product.as_deref())
            && self.matches_optional(
                &self.platform,
                conditions.platform.map(|platform| platform.canonical_tag()),
            )
            && self.matches_optional(&self.host, conditions.host.map(|host| host.canonical_tag()))
            && self.matches_optional(
                &self.runtime,
                conditions.runtime.map(|runtime| runtime.canonical_tag()),
            );

        source_graph_matches && profile_matches
    }

    /// Return true when an optional selector matches one set axis.
    fn matches_set(&self, selector: &Option<ConditionSelector>, names: &IndexSet<String>) -> bool {
        let Some(selector) = selector else {
            return true;
        };

        selector.matches_any(names.iter().map(String::as_str))
    }

    /// Return true when an optional selector matches one scalar axis.
    fn matches_optional(&self, selector: &Option<ConditionSelector>, name: Option<&str>) -> bool {
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

    for name in super::builtin_modes().keys() {
        aliases.insert(name.clone(), ConditionGate::mode(name.clone()));
    }

    for name in super::builtin_roles().keys() {
        aliases.insert(name.clone(), ConditionGate::role(name.clone()));
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ConditionSelector {
    /// Condition names or glob patterns.
    pub patterns: Vec<String>,
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

/// Return true when one glob-like pattern matches text.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}
