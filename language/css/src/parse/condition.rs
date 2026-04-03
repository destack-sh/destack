use super::lightning;
use super::lower::Lowerer;
use crate::{
    ComponentValueList, ConditionOperator, ContainerCondition, ContainerScrollStateQuery,
    ContainerStyleQuery, ContainerUnknownCondition, DeclarationValue, EnvironmentVariable,
    EnvironmentVariableName, FeatureComparison, FeatureName, FeatureValue, LocalNodeId,
    MediaCondition, MediaQualifier, MediaQuery, MediaQueryList, MediaType, MediaUnknownCondition,
    Number, QueryFeature, RatioValue, SupportsCondition, SupportsSelectorCondition,
    SupportsUnknownCondition,
};

impl<'a> Lowerer<'a> {
    /// Lower one Lightning media query list into one owned media query list.
    pub(crate) fn lower_media_query_list(
        &mut self,
        media: &lightning::MediaList<'_>,
    ) -> LocalNodeId<MediaQueryList> {
        let queries = media
            .media_queries
            .iter()
            .map(|query| {
                let query = self.lower_media_query(query);

                self.insert_inner(query)
            })
            .collect();

        self.insert_inner(MediaQueryList { queries })
    }

    /// Lower one Lightning media query.
    pub(crate) fn lower_media_query(&mut self, query: &lightning::MediaQuery<'_>) -> MediaQuery {
        MediaQuery {
            qualifier: query
                .qualifier
                .map(|qualifier| self.lower_media_qualifier(qualifier)),
            media_type: self.lower_media_type(&query.media_type),
            condition: query
                .condition
                .as_ref()
                .map(|condition| self.lower_media_condition(condition)),
        }
    }

    /// Lower one Lightning media qualifier.
    pub(crate) fn lower_media_qualifier(&self, qualifier: lightning::Qualifier) -> MediaQualifier {
        match qualifier {
            lightning::Qualifier::Only => MediaQualifier::Only,
            lightning::Qualifier::Not => MediaQualifier::Not,
        }
    }

    /// Lower one Lightning media type.
    pub(crate) fn lower_media_type(&self, media_type: &lightning::MediaType<'_>) -> MediaType {
        match media_type {
            lightning::MediaType::All => MediaType::All,
            lightning::MediaType::Print => MediaType::Print,
            lightning::MediaType::Screen => MediaType::Screen,
            lightning::MediaType::Custom(value) => MediaType::Custom(value.to_string()),
        }
    }

    /// Lower one Lightning media condition.
    pub(crate) fn lower_media_condition(
        &mut self,
        condition: &lightning::MediaCondition<'_>,
    ) -> LocalNodeId<MediaCondition> {
        let condition = match condition {
            lightning::MediaCondition::Feature(feature) => {
                MediaCondition::Feature(self.lower_media_feature(feature))
            }
            lightning::MediaCondition::Not(condition) => {
                MediaCondition::Not(self.lower_media_condition(condition))
            }
            lightning::MediaCondition::Operation {
                operator,
                conditions,
            } => MediaCondition::Operation {
                operator: self.lower_condition_operator(*operator),
                conditions: conditions
                    .iter()
                    .map(|condition| self.lower_media_condition(condition))
                    .collect(),
            },
            lightning::MediaCondition::Unknown(tokens) => {
                MediaCondition::Unknown(MediaUnknownCondition {
                    components: self.lower_component_value_token_list(tokens),
                })
            }
        };

        self.insert_inner(condition)
    }

    /// Lower one Lightning supports condition into one owned supports condition node.
    pub(crate) fn lower_supports_condition(
        &mut self,
        condition: &lightning::SupportsCondition<'_>,
    ) -> LocalNodeId<SupportsCondition> {
        let condition = match condition {
            lightning::SupportsCondition::Not(condition) => {
                SupportsCondition::Not(self.lower_supports_condition(condition))
            }
            lightning::SupportsCondition::And(conditions) => SupportsCondition::And(
                conditions
                    .iter()
                    .map(|condition| self.lower_supports_condition(condition))
                    .collect(),
            ),
            lightning::SupportsCondition::Or(conditions) => SupportsCondition::Or(
                conditions
                    .iter()
                    .map(|condition| self.lower_supports_condition(condition))
                    .collect(),
            ),
            lightning::SupportsCondition::Declaration { property_id, value } => {
                SupportsCondition::Declaration {
                    property: self.lower_property_name_source(property_id.name()),
                    value: self.lower_declaration_value_source(value),
                }
            }
            lightning::SupportsCondition::Selector(selector) => {
                SupportsCondition::Selector(SupportsSelectorCondition {
                    selectors: self.lower_selector_list_source(selector),
                })
            }
            lightning::SupportsCondition::Unknown(condition) => {
                SupportsCondition::Unknown(SupportsUnknownCondition {
                    components: self.lower_component_value_list_source(condition),
                })
            }
        };

        self.insert_inner(condition)
    }

    /// Lower one Lightning container condition into one owned container condition.
    pub(crate) fn lower_container_condition(
        &mut self,
        condition: &lightning::ContainerCondition<'_>,
    ) -> LocalNodeId<ContainerCondition> {
        let condition = match condition {
            lightning::ContainerCondition::Feature(feature) => {
                ContainerCondition::Feature(self.lower_container_feature(feature))
            }
            lightning::ContainerCondition::Not(condition) => {
                ContainerCondition::Not(self.lower_container_condition(condition))
            }
            lightning::ContainerCondition::Operation {
                operator,
                conditions,
            } => ContainerCondition::Operation {
                operator: self.lower_condition_operator(*operator),
                conditions: conditions
                    .iter()
                    .map(|condition| self.lower_container_condition(condition))
                    .collect(),
            },
            lightning::ContainerCondition::Style(query) => {
                ContainerCondition::Style(self.lower_container_style_query(query))
            }
            lightning::ContainerCondition::ScrollState(query) => {
                ContainerCondition::ScrollState(self.lower_container_scroll_state_query(query))
            }
            lightning::ContainerCondition::Unknown(tokens) => {
                ContainerCondition::Unknown(ContainerUnknownCondition {
                    components: self.lower_component_value_token_list(tokens),
                })
            }
        };

        self.insert_inner(condition)
    }

    /// Lower one Lightning condition operator.
    pub(crate) fn lower_condition_operator(
        &self,
        operator: lightning::Operator,
    ) -> ConditionOperator {
        match operator {
            lightning::Operator::And => ConditionOperator::And,
            lightning::Operator::Or => ConditionOperator::Or,
        }
    }

    /// Lower one Lightning feature comparison.
    pub(crate) fn lower_feature_comparison(
        &self,
        comparison: lightning::MediaFeatureComparison,
    ) -> FeatureComparison {
        match comparison {
            lightning::MediaFeatureComparison::Equal => FeatureComparison::Equal,
            lightning::MediaFeatureComparison::GreaterThan => FeatureComparison::GreaterThan,
            lightning::MediaFeatureComparison::GreaterThanEqual => {
                FeatureComparison::GreaterThanEqual
            }
            lightning::MediaFeatureComparison::LessThan => FeatureComparison::LessThan,
            lightning::MediaFeatureComparison::LessThanEqual => FeatureComparison::LessThanEqual,
        }
    }

    /// Lower one Lightning feature name.
    pub(crate) fn lower_feature_name<FeatureId>(
        &mut self,
        name: &lightning::MediaFeatureName<'_, FeatureId>,
    ) -> LocalNodeId<FeatureName>
    where
        FeatureId: lightning::ToCss,
    {
        let name = match name {
            lightning::MediaFeatureName::Standard(value) => {
                FeatureName::Standard(self.serialize_value(value))
            }
            lightning::MediaFeatureName::Custom(value) => {
                FeatureName::Custom(value.as_ref().to_string())
            }
            lightning::MediaFeatureName::Unknown(value) => {
                FeatureName::Unknown(value.as_ref().to_string())
            }
        };

        self.insert_inner(name)
    }

    /// Lower one Lightning feature value.
    pub(crate) fn lower_feature_value(
        &mut self,
        value: &lightning::MediaFeatureValue<'_>,
    ) -> LocalNodeId<FeatureValue> {
        let value = match value {
            lightning::MediaFeatureValue::Length(length) => {
                FeatureValue::Length(DeclarationValue {
                    components: self.lower_length(length),
                })
            }
            lightning::MediaFeatureValue::Number(value) => FeatureValue::Number(Number {
                has_sign: value.is_sign_negative(),
                value: *value,
                integer_value: None,
            }),
            lightning::MediaFeatureValue::Integer(value) => FeatureValue::Integer(*value),
            lightning::MediaFeatureValue::Boolean(value) => FeatureValue::Boolean(*value),
            lightning::MediaFeatureValue::Resolution(value) => {
                FeatureValue::Resolution(DeclarationValue {
                    components: ComponentValueList {
                        values: vec![self.lower_resolution_value(value)],
                    },
                })
            }
            lightning::MediaFeatureValue::Ratio(value) => {
                let ratio = self.insert_inner(RatioValue {
                    numerator: Number {
                        has_sign: value.0.is_sign_negative(),
                        value: value.0,
                        integer_value: None,
                    },
                    denominator: Number {
                        has_sign: value.1.is_sign_negative(),
                        value: value.1,
                        integer_value: None,
                    },
                });

                FeatureValue::Ratio(ratio)
            }
            lightning::MediaFeatureValue::Ident(value) => FeatureValue::Ident(value.to_string()),
            lightning::MediaFeatureValue::Env(value) => {
                FeatureValue::EnvironmentVariable(self.lower_environment_variable(value))
            }
        };

        self.insert_inner(value)
    }

    /// Lower one Lightning environment variable.
    pub(crate) fn lower_environment_variable(
        &mut self,
        value: &lightning::EnvironmentVariable<'_>,
    ) -> LocalNodeId<EnvironmentVariable> {
        let value = EnvironmentVariable {
            name: self.lower_environment_variable_name(&value.name),
            indices: value.indices.clone(),
            fallback: value
                .fallback
                .as_ref()
                .map(|fallback| self.lower_component_value_token_list(fallback)),
        };

        self.insert_inner(value)
    }

    /// Lower one Lightning environment variable name.
    pub(crate) fn lower_environment_variable_name(
        &self,
        name: &lightning::EnvironmentVariableName<'_>,
    ) -> EnvironmentVariableName {
        match name {
            lightning::EnvironmentVariableName::UA(value) => {
                EnvironmentVariableName::Ua(value.as_str().to_string())
            }
            lightning::EnvironmentVariableName::Custom(value) => {
                EnvironmentVariableName::Custom(value.ident.to_string())
            }
            lightning::EnvironmentVariableName::Unknown(value) => {
                EnvironmentVariableName::Unknown(value.0.to_string())
            }
        }
    }

    /// Lower one Lightning query feature.
    pub(crate) fn lower_query_feature<FeatureId>(
        &mut self,
        feature: &lightning::QueryFeature<'_, FeatureId>,
    ) -> LocalNodeId<QueryFeature>
    where
        FeatureId: lightning::ToCss,
    {
        let feature = match feature {
            lightning::QueryFeature::Plain { name, value } => QueryFeature::Plain {
                name: self.lower_feature_name(name),
                value: self.lower_feature_value(value),
            },
            lightning::QueryFeature::Boolean { name } => QueryFeature::Boolean {
                name: self.lower_feature_name(name),
            },
            lightning::QueryFeature::Range {
                name,
                operator,
                value,
            } => QueryFeature::Range {
                name: self.lower_feature_name(name),
                operator: self.lower_feature_comparison(*operator),
                value: self.lower_feature_value(value),
            },
            lightning::QueryFeature::Interval {
                name,
                start,
                start_operator,
                end,
                end_operator,
            } => QueryFeature::Interval {
                name: self.lower_feature_name(name),
                start: self.lower_feature_value(start),
                start_operator: self.lower_feature_comparison(*start_operator),
                end: self.lower_feature_value(end),
                end_operator: self.lower_feature_comparison(*end_operator),
            },
        };

        self.insert_inner(feature)
    }

    /// Lower one Lightning media feature.
    pub(crate) fn lower_media_feature(
        &mut self,
        feature: &lightning::MediaFeature<'_>,
    ) -> LocalNodeId<QueryFeature> {
        self.lower_query_feature(feature)
    }

    /// Lower one Lightning container feature.
    pub(crate) fn lower_container_feature(
        &mut self,
        feature: &lightning::ContainerSizeFeature<'_>,
    ) -> LocalNodeId<QueryFeature> {
        self.lower_query_feature(feature)
    }

    /// Lower one Lightning scroll state feature.
    pub(crate) fn lower_scroll_state_feature(
        &mut self,
        feature: &lightning::ScrollStateFeature<'_>,
    ) -> LocalNodeId<QueryFeature> {
        self.lower_query_feature(feature)
    }

    /// Lower one Lightning style query.
    pub(crate) fn lower_container_style_query(
        &mut self,
        query: &lightning::StyleQuery<'_>,
    ) -> LocalNodeId<ContainerStyleQuery> {
        let query = match query {
            lightning::StyleQuery::Declaration(property) => ContainerStyleQuery::Declaration {
                property: self.lower_property_name_source(property.property_id().name()),
                value: self.lower_declaration_value_property(property),
            },
            lightning::StyleQuery::Property(property) => {
                ContainerStyleQuery::Property(self.lower_property_name_source(property.name()))
            }
            lightning::StyleQuery::Not(query) => {
                ContainerStyleQuery::Not(self.lower_container_style_query(query))
            }
            lightning::StyleQuery::Operation {
                operator,
                conditions,
            } => ContainerStyleQuery::Operation {
                operator: self.lower_condition_operator(*operator),
                conditions: conditions
                    .iter()
                    .map(|condition| self.lower_container_style_query(condition))
                    .collect(),
            },
        };

        self.insert_inner(query)
    }

    /// Lower one Lightning scroll state query.
    pub(crate) fn lower_container_scroll_state_query(
        &mut self,
        query: &lightning::ScrollStateQuery<'_>,
    ) -> LocalNodeId<ContainerScrollStateQuery> {
        let query = match query {
            lightning::ScrollStateQuery::Feature(feature) => {
                ContainerScrollStateQuery::Feature(self.lower_scroll_state_feature(feature))
            }
            lightning::ScrollStateQuery::Not(query) => {
                ContainerScrollStateQuery::Not(self.lower_container_scroll_state_query(query))
            }
            lightning::ScrollStateQuery::Operation {
                operator,
                conditions,
            } => ContainerScrollStateQuery::Operation {
                operator: self.lower_condition_operator(*operator),
                conditions: conditions
                    .iter()
                    .map(|condition| self.lower_container_scroll_state_query(condition))
                    .collect(),
            },
        };

        self.insert_inner(query)
    }
}
