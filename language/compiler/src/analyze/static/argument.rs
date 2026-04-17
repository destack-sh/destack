use std::fmt;

use destack_core::StringId;
use destack_dir::{StaticArgument, StaticExpression};

/// A structured error for static argument resolution.
#[derive(Debug, Clone)]
pub(crate) struct StaticArgumentError {
    /// The error context.
    context: String,
    /// The error kind.
    kind: StaticArgumentErrorKind,
}

/// The error kind for static argument resolution.
#[derive(Debug, Clone)]
enum StaticArgumentErrorKind {
    /// Static arguments were not evaluated.
    Unevaluated,
    /// Named and positional arguments were mixed.
    MixedNamedAndPositional,
    /// An expected argument was missing.
    MissingArgument { name: String },
    /// An unexpected argument was provided.
    UnexpectedArgument { allowed: Vec<String> },
    /// The argument list had the wrong arity.
    WrongArity { expected: usize, found: usize },
    /// The argument list was too short.
    InsufficientArguments { expected: usize, found: usize },
}

impl StaticArgumentError {
    /// Create a new error with context.
    fn new(context: &str, kind: StaticArgumentErrorKind) -> Self {
        Self {
            context: context.to_string(),
            kind,
        }
    }
}

impl fmt::Display for StaticArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            StaticArgumentErrorKind::Unevaluated => {
                write!(f, "{} requires evaluated static arguments", self.context)
            }
            StaticArgumentErrorKind::MixedNamedAndPositional => write!(
                f,
                "{} named arguments cannot be mixed with positional ones",
                self.context
            ),
            StaticArgumentErrorKind::MissingArgument { name } => {
                write!(f, "{} is missing static argument {name}", self.context)
            }
            StaticArgumentErrorKind::UnexpectedArgument { allowed } => {
                let allowed = allowed.join(" and ");
                write!(
                    f,
                    "{} only supports static arguments {allowed}",
                    self.context
                )
            }
            StaticArgumentErrorKind::WrongArity { expected, found } => write!(
                f,
                "{} expects exactly {expected} static arguments, got {found}",
                self.context
            ),
            StaticArgumentErrorKind::InsufficientArguments { expected, found } => write!(
                f,
                "{} expects at least {expected} static arguments, got {found}",
                self.context
            ),
        }
    }
}

impl std::error::Error for StaticArgumentError {}

/// A resolver for evaluated static arguments.
pub(crate) struct StaticArgumentResolver<'a> {
    /// The error context.
    context: &'a str,
    /// The evaluated arguments.
    arguments: Vec<ResolvedStaticArgument<'a>>,
    /// Whether arguments are named.
    has_named: bool,
}

/// A resolved static argument.
struct ResolvedStaticArgument<'a> {
    /// The argument name when present.
    name: Option<StringId>,
    /// The argument value.
    value: &'a StaticExpression,
}

impl<'a> StaticArgumentResolver<'a> {
    /// Build a resolver for evaluated static arguments.
    pub(crate) fn new(
        generic_arguments: &'a [StaticArgument],
        context: &'a str,
    ) -> Result<Self, StaticArgumentError> {
        // collect evaluated arguments
        let mut arguments = Vec::with_capacity(generic_arguments.len());
        let mut has_named = false;
        let mut has_positional = false;
        for argument in generic_arguments {
            let StaticArgument::Evaluated { name, value } = argument else {
                return Err(StaticArgumentError::new(
                    context,
                    StaticArgumentErrorKind::Unevaluated,
                ));
            };

            // record argument shape
            if name.is_some() {
                has_named = true;
            } else {
                has_positional = true;
            }

            // store the evaluated argument
            arguments.push(ResolvedStaticArgument { name: *name, value });
        }

        // reject mixed argument styles
        if has_named && has_positional {
            return Err(StaticArgumentError::new(
                context,
                StaticArgumentErrorKind::MixedNamedAndPositional,
            ));
        }

        // return the resolver
        Ok(Self {
            context,
            arguments,
            has_named,
        })
    }

    /// Resolve a static argument by name or index.
    pub(crate) fn argument_by_name_or_index(
        &self,
        name: (StringId, &str),
        index: usize,
    ) -> Result<&'a StaticExpression, StaticArgumentError> {
        // resolve named arguments when present
        if self.has_named {
            // search for the named argument
            for argument in &self.arguments {
                if argument.name == Some(name.0) {
                    return Ok(argument.value);
                }
            }

            // report missing named arguments
            return Err(StaticArgumentError::new(
                self.context,
                StaticArgumentErrorKind::MissingArgument {
                    name: name.1.to_string(),
                },
            ));
        }

        // resolve positional arguments by index
        if index >= self.arguments.len() {
            return Err(StaticArgumentError::new(
                self.context,
                StaticArgumentErrorKind::InsufficientArguments {
                    expected: index + 1,
                    found: self.arguments.len(),
                },
            ));
        }

        // return the positional argument
        Ok(self.arguments[index].value)
    }

    /// Ensure the argument list has exactly the given arity.
    pub(crate) fn ensure_exact_arity(&self, expected: usize) -> Result<(), StaticArgumentError> {
        // enforce exact arity
        if self.arguments.len() != expected {
            return Err(StaticArgumentError::new(
                self.context,
                StaticArgumentErrorKind::WrongArity {
                    expected,
                    found: self.arguments.len(),
                },
            ));
        }

        // accept the arity
        Ok(())
    }

    /// Ensure no unexpected named arguments are present.
    pub(crate) fn ensure_only_names(
        &self,
        allowed: &[(StringId, &str)],
    ) -> Result<(), StaticArgumentError> {
        // allow positional arguments without name validation
        if !self.has_named {
            return Ok(());
        }

        // collect allowed names
        let mut allowed_names = Vec::with_capacity(allowed.len());
        for (name_id, name) in allowed {
            allowed_names.push((*name_id, name.to_string()));
        }

        // reject unexpected names
        for argument in &self.arguments {
            let Some(name) = argument.name else {
                continue;
            };

            if allowed_names
                .iter()
                .any(|(allowed_id, _)| *allowed_id == name)
            {
                continue;
            }

            // report unexpected named arguments
            let allowed = allowed_names.iter().map(|(_, name)| name.clone()).collect();
            return Err(StaticArgumentError::new(
                self.context,
                StaticArgumentErrorKind::UnexpectedArgument { allowed },
            ));
        }

        Ok(())
    }
}
