use super::printer::Printer;
use crate::{
    ConditionOperator, ContainerCondition, ContainerName, ContainerScrollStateQuery,
    ContainerStyleQuery, EnvironmentVariable, EnvironmentVariableName, FeatureComparison,
    FeatureName, FeatureValue, ImportLayer, LayerNameList, LocalNodeId, MediaCondition,
    MediaQualifier, MediaQuery, MediaQueryList, MediaType, PropertyName, QueryFeature, RatioValue,
    SupportsCondition,
};

use crate::Tree;

/// Print one layer name list as canonical CSS source.
pub fn print_layer_name_list(name: &LayerNameList) -> String {
    Printer::render_layer_name_list(name)
}

/// Print one media query list as canonical CSS source.
pub fn print_media_query_list(tree: &Tree, media: LocalNodeId<MediaQueryList>) -> String {
    Printer::new(tree).render_media_query_list(media)
}

/// Print one supports condition as canonical CSS source.
pub fn print_supports_condition(tree: &Tree, condition: LocalNodeId<SupportsCondition>) -> String {
    Printer::new(tree).render_supports_condition(condition)
}

impl<'a> Printer<'a> {
    /// Render one property name as canonical CSS source.
    pub(crate) fn render_property_name(property_name: &PropertyName) -> &str {
        match property_name {
            PropertyName::Standard(name) | PropertyName::Custom(name) => name,
        }
    }

    /// Render one layer name list as canonical CSS source.
    pub(crate) fn render_layer_name_list(name: &LayerNameList) -> String {
        Self::join_sources(".", &name.names, |name| name.clone())
    }

    /// Render one import layer clause as canonical CSS source.
    pub(crate) fn render_import_layer(layer: &ImportLayer) -> String {
        let mut source = String::from("layer");

        if let Some(name) = &layer.name {
            source.push('(');
            source.push_str(&Self::render_layer_name_list(name));
            source.push(')');
        }

        source
    }

    /// Render one container name as canonical CSS source.
    pub(crate) fn render_container_name(name: &ContainerName) -> &str {
        &name.name
    }

    /// Render one media query list as canonical CSS source.
    pub(crate) fn render_media_query_list(&self, media: LocalNodeId<MediaQueryList>) -> String {
        let media = self.tree.get(media);

        Self::join_sources(", ", &media.queries, |query| {
            self.render_media_query(self.tree.get(*query))
        })
    }

    /// Render one media query as canonical CSS source.
    pub(crate) fn render_media_query(&self, query: &MediaQuery) -> String {
        let mut source = String::new();

        if let Some(qualifier) = query.qualifier {
            source.push_str(Self::render_media_qualifier(qualifier));
            source.push(' ');
        }

        source.push_str(&Self::render_media_type(&query.media_type));

        if let Some(condition) = query.condition {
            let condition = self.render_media_condition(condition);

            if !condition.is_empty() {
                if !source.is_empty() {
                    source.push_str(" and ");
                }

                source.push_str(&condition);
            }
        }

        source
    }

    /// Render one media qualifier as canonical CSS source.
    pub(crate) fn render_media_qualifier(qualifier: MediaQualifier) -> &'static str {
        match qualifier {
            MediaQualifier::Only => "only",
            MediaQualifier::Not => "not",
        }
    }

    /// Render one media type as canonical CSS source.
    pub(crate) fn render_media_type(media_type: &MediaType) -> String {
        match media_type {
            MediaType::All => "all".to_string(),
            MediaType::Print => "print".to_string(),
            MediaType::Screen => "screen".to_string(),
            MediaType::Custom(value) => value.clone(),
        }
    }

    /// Render one media condition as canonical CSS source.
    pub(crate) fn render_media_condition(&self, condition: LocalNodeId<MediaCondition>) -> String {
        let condition = self.tree.get(condition);

        match condition {
            MediaCondition::Feature(feature) => self.render_query_feature(*feature),
            MediaCondition::Not(condition) => {
                format!("not {}", self.render_parenthesized_media(*condition))
            }
            MediaCondition::Operation {
                operator,
                conditions,
            } => Self::join_conditions(
                Self::render_condition_operator_separator(*operator),
                conditions,
                |condition| self.render_parenthesized_media(*condition),
            ),
            MediaCondition::Unknown(condition) => {
                self.render_component_value_list(&condition.components)
            }
        }
    }

    /// Render one supports condition as canonical CSS source.
    pub(crate) fn render_supports_condition(
        &self,
        condition: LocalNodeId<SupportsCondition>,
    ) -> String {
        let condition = self.tree.get(condition);

        match condition {
            SupportsCondition::Not(condition) => {
                format!("not {}", self.render_parenthesized_supports(*condition))
            }
            SupportsCondition::And(conditions) => {
                Self::join_conditions(" and ", conditions, |condition| {
                    self.render_parenthesized_supports(*condition)
                })
            }
            SupportsCondition::Or(conditions) => {
                Self::join_conditions(" or ", conditions, |condition| {
                    self.render_parenthesized_supports(*condition)
                })
            }
            SupportsCondition::Declaration { property, value } => {
                format!(
                    "({}: {})",
                    Self::render_property_name(property),
                    self.render_component_value_list(value.components()),
                )
            }
            SupportsCondition::Selector(selector) => {
                format!(
                    "selector({})",
                    self.render_selector_list(selector.selectors)
                )
            }
            SupportsCondition::Unknown(condition) => {
                self.render_component_value_list(&condition.components)
            }
        }
    }

    /// Render one container condition as canonical CSS source.
    pub(crate) fn render_container_condition(
        &self,
        condition: LocalNodeId<ContainerCondition>,
    ) -> String {
        match self.tree.get(condition) {
            ContainerCondition::Feature(feature) => self.render_query_feature(*feature),
            ContainerCondition::Not(condition) => {
                format!("not {}", self.render_parenthesized_container(*condition))
            }
            ContainerCondition::Operation {
                operator,
                conditions,
            } => Self::join_conditions(
                Self::render_condition_operator_separator(*operator),
                conditions,
                |condition| self.render_parenthesized_container(*condition),
            ),
            ContainerCondition::Style(query) => {
                format!("style({})", self.render_container_style_query(*query))
            }
            ContainerCondition::ScrollState(query) => {
                format!(
                    "scroll-state({})",
                    self.render_container_scroll_state_query(*query)
                )
            }
            ContainerCondition::Unknown(condition) => {
                self.render_component_value_list(&condition.components)
            }
        }
    }

    /// Render one condition operator separator.
    pub(crate) fn render_condition_operator_separator(operator: ConditionOperator) -> &'static str {
        match operator {
            ConditionOperator::And => " and ",
            ConditionOperator::Or => " or ",
        }
    }

    /// Render one feature comparison as canonical CSS source.
    pub(crate) fn render_feature_comparison(comparison: FeatureComparison) -> &'static str {
        match comparison {
            FeatureComparison::Equal => " = ",
            FeatureComparison::GreaterThan => " > ",
            FeatureComparison::GreaterThanEqual => " >= ",
            FeatureComparison::LessThan => " < ",
            FeatureComparison::LessThanEqual => " <= ",
        }
    }

    /// Render one feature name as canonical CSS source.
    pub(crate) fn render_feature_name(&self, name: LocalNodeId<FeatureName>) -> String {
        match self.tree.get(name) {
            FeatureName::Standard(name)
            | FeatureName::Custom(name)
            | FeatureName::Unknown(name) => name.clone(),
        }
    }

    /// Render one feature value as canonical CSS source.
    pub(crate) fn render_feature_value(&self, value: LocalNodeId<FeatureValue>) -> String {
        let value = self.tree.get(value);

        match value {
            FeatureValue::Length(value) | FeatureValue::Resolution(value) => {
                self.render_component_value_list(value.components())
            }
            FeatureValue::Number(value) => Self::render_number(*value),
            FeatureValue::Integer(value) => value.to_string(),
            FeatureValue::Boolean(value) => {
                if *value {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            }
            FeatureValue::Ratio(value) => self.render_ratio_value(*value),
            FeatureValue::Ident(value) => value.clone(),
            FeatureValue::EnvironmentVariable(value) => self.render_environment_variable(*value),
        }
    }

    /// Render one ratio value as canonical CSS source.
    pub(crate) fn render_ratio_value(&self, value: LocalNodeId<RatioValue>) -> String {
        let value = self.tree.get(value);

        format!(
            "{} / {}",
            Self::render_number(value.numerator),
            Self::render_number(value.denominator),
        )
    }

    /// Render one environment variable as canonical CSS source.
    pub(crate) fn render_environment_variable(
        &self,
        value: LocalNodeId<EnvironmentVariable>,
    ) -> String {
        let value = self.tree.get(value);
        let mut source = String::from("env(");

        source.push_str(Self::render_environment_variable_name(&value.name));

        for index in &value.indices {
            source.push(' ');
            source.push_str(&index.to_string());
        }

        if let Some(fallback) = &value.fallback {
            source.push_str(", ");
            source.push_str(&self.render_component_value_list(fallback));
        }

        source.push(')');
        source
    }

    /// Render one environment variable name as canonical CSS source.
    pub(crate) fn render_environment_variable_name(name: &EnvironmentVariableName) -> &str {
        match name {
            EnvironmentVariableName::Ua(value)
            | EnvironmentVariableName::Custom(value)
            | EnvironmentVariableName::Unknown(value) => value,
        }
    }

    /// Render one query feature as canonical CSS source.
    pub(crate) fn render_query_feature(&self, feature: LocalNodeId<QueryFeature>) -> String {
        let feature = self.tree.get(feature);

        match feature {
            QueryFeature::Plain { name, value } => {
                format!(
                    "({}: {})",
                    self.render_feature_name(*name),
                    self.render_feature_value(*value)
                )
            }
            QueryFeature::Boolean { name } => format!("({})", self.render_feature_name(*name)),
            QueryFeature::Range {
                name,
                operator,
                value,
            } => format!(
                "({}{}{})",
                self.render_feature_name(*name),
                Self::render_feature_comparison(*operator),
                self.render_feature_value(*value)
            ),
            QueryFeature::Interval {
                name,
                start,
                start_operator,
                end,
                end_operator,
            } => format!(
                "({}{}{}{}{})",
                self.render_feature_value(*start),
                Self::render_feature_comparison(*start_operator),
                self.render_feature_name(*name),
                Self::render_feature_comparison(*end_operator),
                self.render_feature_value(*end)
            ),
        }
    }

    /// Render one style query as canonical CSS source.
    pub(crate) fn render_container_style_query(
        &self,
        query: LocalNodeId<ContainerStyleQuery>,
    ) -> String {
        let query = self.tree.get(query);

        match query {
            ContainerStyleQuery::Declaration { property, value } => {
                format!(
                    "({}: {})",
                    Self::render_property_name(property),
                    self.render_component_value_list(value.components())
                )
            }
            ContainerStyleQuery::Property(property) => {
                format!("({})", Self::render_property_name(property))
            }
            ContainerStyleQuery::Not(query) => {
                format!("not {}", self.render_parenthesized_style_query(*query))
            }
            ContainerStyleQuery::Operation {
                operator,
                conditions,
            } => Self::join_conditions(
                Self::render_condition_operator_separator(*operator),
                conditions,
                |query| self.render_parenthesized_style_query(*query),
            ),
        }
    }

    /// Render one scroll state query as canonical CSS source.
    pub(crate) fn render_container_scroll_state_query(
        &self,
        query: LocalNodeId<ContainerScrollStateQuery>,
    ) -> String {
        let query = self.tree.get(query);

        match query {
            ContainerScrollStateQuery::Feature(feature) => self.render_query_feature(*feature),
            ContainerScrollStateQuery::Not(query) => {
                format!(
                    "not {}",
                    self.render_parenthesized_scroll_state_query(*query)
                )
            }
            ContainerScrollStateQuery::Operation {
                operator,
                conditions,
            } => Self::join_conditions(
                Self::render_condition_operator_separator(*operator),
                conditions,
                |query| self.render_parenthesized_scroll_state_query(*query),
            ),
        }
    }

    /// Join recursive conditions with one boolean operator.
    pub(crate) fn join_conditions<T>(
        separator: &str,
        conditions: &[T],
        parenthesize: impl Fn(&T) -> String,
    ) -> String {
        let mut source = String::new();

        for (index, condition) in conditions.iter().enumerate() {
            if index > 0 {
                source.push_str(separator);
            }

            source.push_str(&parenthesize(condition));
        }

        source
    }

    /// Join one list of authored sources.
    pub(crate) fn join_sources<T>(
        separator: &str,
        values: &[T],
        to_source: impl Fn(&T) -> String,
    ) -> String {
        let mut source = String::new();

        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                source.push_str(separator);
            }

            source.push_str(&to_source(value));
        }

        source
    }

    /// Parenthesize one media condition when needed.
    pub(crate) fn render_parenthesized_media(
        &self,
        condition: LocalNodeId<MediaCondition>,
    ) -> String {
        match self.tree.get(condition) {
            MediaCondition::Feature(_) | MediaCondition::Unknown(_) => {
                self.render_media_condition(condition)
            }
            MediaCondition::Not(_) | MediaCondition::Operation { .. } => {
                format!("({})", self.render_media_condition(condition))
            }
        }
    }

    /// Parenthesize one supports condition when needed.
    pub(crate) fn render_parenthesized_supports(
        &self,
        condition: LocalNodeId<SupportsCondition>,
    ) -> String {
        match self.tree.get(condition) {
            SupportsCondition::Declaration { .. }
            | SupportsCondition::Selector(_)
            | SupportsCondition::Unknown(_) => self.render_supports_condition(condition),
            SupportsCondition::Not(_) | SupportsCondition::And(_) | SupportsCondition::Or(_) => {
                format!("({})", self.render_supports_condition(condition))
            }
        }
    }

    /// Parenthesize one container condition when needed.
    pub(crate) fn render_parenthesized_container(
        &self,
        condition: LocalNodeId<ContainerCondition>,
    ) -> String {
        match self.tree.get(condition) {
            ContainerCondition::Feature(_)
            | ContainerCondition::Style(_)
            | ContainerCondition::ScrollState(_)
            | ContainerCondition::Unknown(_) => self.render_container_condition(condition),
            ContainerCondition::Not(_) | ContainerCondition::Operation { .. } => {
                format!("({})", self.render_container_condition(condition))
            }
        }
    }

    /// Parenthesize one style query when needed.
    pub(crate) fn render_parenthesized_style_query(
        &self,
        query: LocalNodeId<ContainerStyleQuery>,
    ) -> String {
        match self.tree.get(query) {
            ContainerStyleQuery::Declaration { .. } | ContainerStyleQuery::Property(_) => {
                self.render_container_style_query(query)
            }
            ContainerStyleQuery::Not(_) | ContainerStyleQuery::Operation { .. } => {
                format!("({})", self.render_container_style_query(query))
            }
        }
    }

    /// Parenthesize one scroll state query when needed.
    pub(crate) fn render_parenthesized_scroll_state_query(
        &self,
        query: LocalNodeId<ContainerScrollStateQuery>,
    ) -> String {
        match self.tree.get(query) {
            ContainerScrollStateQuery::Feature(_) => {
                self.render_container_scroll_state_query(query)
            }
            ContainerScrollStateQuery::Not(_) | ContainerScrollStateQuery::Operation { .. } => {
                format!("({})", self.render_container_scroll_state_query(query))
            }
        }
    }
}
