use tspp_dir as dir;
use tspp_repository::ProviderError;

use super::DirModule;

/// One kind of test registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestRegistrationKind {
    /// One test case.
    Case,
    /// One test suite.
    Suite,
}

/// One canonical test registration call.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestRegistration<'a> {
    /// The authored call arguments.
    pub(crate) arguments: &'a [dir::LocalNodeId<dir::Argument>],
    /// The registered test kind.
    pub(crate) kind: TestRegistrationKind,
}

/// One canonical test hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TestHook {
    /// Setup before all tests in a suite.
    BeforeAll,
    /// Cleanup after all tests in a suite.
    AfterAll,
    /// A function around all tests in a suite.
    AroundAll,
    /// Setup before each test in a suite.
    BeforeEach,
    /// Cleanup after each test in a suite.
    AfterEach,
    /// A function around each test in a suite.
    AroundEach,
}

/// One canonical test hook call.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestHookCall {
    /// The scoped test registration receiver, if any.
    pub(crate) receiver: Option<dir::LocalNodeId<dir::Expression>>,
    /// The registered hook kind.
    pub(crate) hook: TestHook,
}

/// The lexical suite containing one test API call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TestSuiteScope {
    /// The source module's implicit root suite.
    Module,
    /// One explicit suite registration.
    Registration(dir::LocalNodeId<dir::Expression>),
}

impl TestHook {
    /// Return the hook represented by one canonical function.
    fn from_item(item: dir::LanguageItem) -> Option<Self> {
        match item {
            dir::LanguageItem::TestBeforeAll => Some(Self::BeforeAll),
            dir::LanguageItem::TestAfterAll => Some(Self::AfterAll),
            dir::LanguageItem::TestAroundAll => Some(Self::AroundAll),
            dir::LanguageItem::TestBeforeEach => Some(Self::BeforeEach),
            dir::LanguageItem::TestAfterEach => Some(Self::AfterEach),
            dir::LanguageItem::TestAroundEach => Some(Self::AroundEach),
            _ => None,
        }
    }

    /// Return the hook represented by one canonical Test member.
    fn from_member(member: dir::LanguageMember) -> Option<Self> {
        if member.owner != dir::LanguageItem::Test {
            return None;
        }

        // match the canonical Test member name
        [
            Self::BeforeAll,
            Self::AfterAll,
            Self::AroundAll,
            Self::BeforeEach,
            Self::AfterEach,
            Self::AroundEach,
        ]
        .into_iter()
        .find(|hook| member.key == dir::LanguageItem::Test.member(hook.name()).key)
    }

    /// Return the canonical hook name.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::BeforeAll => "beforeAll",
            Self::AfterAll => "afterAll",
            Self::AroundAll => "aroundAll",
            Self::BeforeEach => "beforeEach",
            Self::AfterEach => "afterEach",
            Self::AroundEach => "aroundEach",
        }
    }
}

impl TestRegistrationKind {
    /// Return the registration kind represented by one canonical interface.
    fn from_item(item: dir::LanguageItem) -> Option<Self> {
        match item {
            dir::LanguageItem::Test
            | dir::LanguageItem::ParameterizedTest
            | dir::LanguageItem::TableTest => Some(Self::Case),
            dir::LanguageItem::TestSuite
            | dir::LanguageItem::ParameterizedSuite
            | dir::LanguageItem::TableSuite => Some(Self::Suite),
            _ => None,
        }
    }
}

impl DirModule<'_> {
    /// Return one canonical test registration call.
    pub(crate) fn test_registration(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<TestRegistration<'_>>, ProviderError> {
        // select one call and its canonical registration representation
        let dir::Expression::Call {
            left, arguments, ..
        } = self.view().get(expression)
        else {
            return Ok(None);
        };
        let Some(item) = self.representation_item(left.into_any())? else {
            return Ok(None);
        };

        let Some(kind) = TestRegistrationKind::from_item(item) else {
            return Ok(None);
        };

        Ok(Some(TestRegistration { arguments, kind }))
    }

    /// Return the receiver of one canonical test registration modifier.
    pub(crate) fn test_modifier(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        name: &str,
    ) -> Result<Option<dir::LocalNodeId<dir::Expression>>, ProviderError> {
        let dir::Expression::Member { left, .. } = self.view().get(expression) else {
            return Ok(None);
        };
        let name = dir::StaticKey::Name(dir::StringId::for_text(name));

        // require the named member of one canonical registration interface
        let Some(member) = self.language_member(expression)? else {
            return Ok(None);
        };
        if TestRegistrationKind::from_item(member.owner).is_none() || member.key != name {
            return Ok(None);
        }

        Ok(Some(*left))
    }

    /// Return one enabled boolean registration option.
    pub(crate) fn test_option(
        &self,
        registration: TestRegistration<'_>,
        name: &str,
    ) -> Option<(
        dir::LocalNodeId<dir::Property>,
        dir::LocalNodeId<dir::Expression>,
    )> {
        // select the options object in the second argument position
        let argument = registration.arguments.get(1)?;
        let options = self.view().get(*argument).value()?;
        let dir::Expression::ObjectExpression { properties } = self.view().get(options) else {
            return None;
        };
        let key = dir::StaticKey::Name(dir::StringId::for_text(name));
        let field = self.direct_field(properties, key)?;

        // require the literal enabled value
        if self.view().get(field.1).as_scalar() != Some(dir::Literal::Boolean(true)) {
            return None;
        }

        Some(field)
    }

    /// Return one canonical test hook call.
    pub(crate) fn test_hook(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<TestHookCall>, ProviderError> {
        // recognize top level hook functions
        let direct = self
            .language_item(expression)?
            .and_then(TestHook::from_item);
        if let Some(hook) = direct {
            return Ok(Some(TestHookCall {
                receiver: None,
                hook,
            }));
        }

        // recognize hooks scoped to one Test value
        let Some(call) = self.member_call(expression) else {
            return Ok(None);
        };
        let Some(member) = self.language_member(expression)? else {
            return Ok(None);
        };
        let Some(hook) = TestHook::from_member(member) else {
            return Ok(None);
        };

        Ok(Some(TestHookCall {
            receiver: Some(call.receiver),
            hook,
        }))
    }

    /// Return the lexical test suite containing one node.
    pub(crate) fn test_suite_scope(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<TestSuiteScope>, ProviderError> {
        let view = self.view();
        let mut current = node;

        // select the first callable containing the node
        while let Some(parent) = view.get_parent_any(current) {
            if self.callable_body(parent).is_none() {
                current = parent;
                continue;
            }

            // select one inline lambda passed directly to a call
            let Some(callback) = view.get_parent_any(parent) else {
                return Ok(None);
            };
            let Ok(callback) = callback.try_into_typed::<dir::Expression>() else {
                return Ok(None);
            };
            if self.lambda(callback).is_none() {
                return Ok(None);
            }
            let Some(expression) = self.argument_call(callback) else {
                return Ok(None);
            };

            // require one suite registration call
            let Some(registration) = self.test_registration(expression)? else {
                return Ok(None);
            };
            if registration.kind != TestRegistrationKind::Suite {
                return Ok(None);
            }

            return Ok(Some(TestSuiteScope::Registration(expression)));
        }

        Ok(Some(TestSuiteScope::Module))
    }
}
