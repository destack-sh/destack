use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require declarations in the canonical source order.
    pub DECLARATION_ORDER {
        id: "declaration-order",
        summary: "Require declarations in the canonical source order",
        explanation: r#"
Inconsistent declaration order makes source files and type bodies harder to scan predictably.
Instead, you SHOULD order module items as imports, module metadata, globals, constants, types, extensions, functions, exports, and executed expressions.

Type members SHOULD order associated types, constants, static state, instance fields, constructors, static accessors, instance accessors, static methods, and instance methods.
Vitest-style `describe` and `test` registrations are executed expressions, so they remain last in `*.test.ds` files.
"#,
        example: {
            reported: r#"
function run(): void {}

const RETRY_LIMIT = 3;
"#,
            accepted: r#"
const RETRY_LIMIT = 3;

function run(): void {}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The canonical position of one module item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum ModuleOrder {
    /// Import declaration.
    Import,
    /// Module metadata declaration.
    Module,
    /// Global declaration block.
    Global,
    /// Module constant.
    Constant,
    /// Type declaration.
    Type,
    /// Extension declaration.
    Extension,
    /// Function declaration.
    Function,
    /// Export declaration.
    Export,
    /// Module-executed expression.
    Executed,
}

impl ModuleOrder {
    /// Return the source noun for this order.
    const fn noun(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Module => "module declaration",
            Self::Global => "global declaration",
            Self::Constant => "constant",
            Self::Type => "type",
            Self::Extension => "extension",
            Self::Function => "function",
            Self::Export => "export",
            Self::Executed => "executed expression",
        }
    }
}

/// The canonical position of one type member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum MemberOrder {
    /// Associated type.
    AssociatedType,
    /// Associated constant or const block.
    AssociatedConstant,
    /// Static field or initialization block.
    StaticState,
    /// Instance field or index signature.
    InstanceField,
    /// Constructor or construct signature.
    Constructor,
    /// Static getter or setter.
    StaticAccessor,
    /// Instance getter or setter.
    InstanceAccessor,
    /// Static method.
    StaticMethod,
    /// Instance method or call signature.
    InstanceMethod,
}

impl MemberOrder {
    /// Return the source noun for this order.
    const fn noun(self) -> &'static str {
        match self {
            Self::AssociatedType => "associated type",
            Self::AssociatedConstant => "associated constant",
            Self::StaticState => "static state",
            Self::InstanceField => "instance field",
            Self::Constructor => "constructor",
            Self::StaticAccessor => "static accessor",
            Self::InstanceAccessor => "instance accessor",
            Self::StaticMethod => "static method",
            Self::InstanceMethod => "instance method",
        }
    }
}

/// Report module items and type members outside their canonical positions.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect top-level items independently in every contributing source file
    for file in module.files {
        let mut expressions = Vec::new();
        for expression in module.roots {
            let span = module.span(expression.into_any())?;
            if span.file == file.id {
                expressions.push(*expression);
            }
        }
        report_module_order(module, lint, expressions, &mut output)?;
    }

    // inspect global declaration blocks
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        if let dir::Declaration::Global(declaration) = declaration {
            report_module_order(
                module,
                lint,
                declaration.expressions.iter().copied(),
                &mut output,
            )?;
        }

        // inspect nominal declaration members
        if let Some(members) = declaration.member_ids() {
            report_member_order(
                module,
                lint,
                members.iter().map(|member| {
                    member_order(view.get(*member)).map(|order| (member.into_any(), order))
                }),
                &mut output,
            )?;
        }

        // inspect structural interface members
        if let Some(members) = declaration.type_member_ids() {
            report_member_order(
                module,
                lint,
                members.iter().map(|member| {
                    type_member_order(view.get(*member)).map(|order| (member.into_any(), order))
                }),
                &mut output,
            )?;
        }
    }

    Ok(output)
}

/// Report expressions that move before an earlier occupied order.
fn report_module_order(
    module: &DirModule<'_>,
    lint: &Lint,
    expressions: impl IntoIterator<Item = dir::LocalNodeId<dir::Expression>>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let mut furthest: Option<ModuleOrder> = None;

    // compare each item with the furthest preceding category
    for expression in expressions {
        let (order, node) = module_order(expression, module);
        let previous = furthest;
        furthest = Some(furthest.map_or(order, |furthest| furthest.max(order)));
        let Some(previous) = previous else {
            continue;
        };
        if order >= previous {
            continue;
        }

        // anchor exports to their complete declaration and named items to their main span
        let span = if order == ModuleOrder::Export {
            module.source_extent(node)?
        } else {
            module.main_span(node)?
        };
        let message = format!(
            "{} must appear before an earlier {}",
            order.noun(),
            previous.noun()
        );
        output.report(lint.diagnostic(message, span));
    }

    Ok(())
}

/// Return the positions occupied by one module item.
fn module_order(
    expression: dir::LocalNodeId<dir::Expression>,
    module: &DirModule<'_>,
) -> (ModuleOrder, dir::LocalNodeIdAny) {
    let view = module.view();
    let node = match view.get(expression) {
        dir::Expression::Declaration(declaration) => declaration.into_any(),
        _ => expression.into_any(),
    };
    let order = match view.get(expression) {
        dir::Expression::Import { .. } => ModuleOrder::Import,
        dir::Expression::Let {
            kind: dir::LetKind::Const,
            ..
        } => ModuleOrder::Constant,
        dir::Expression::Declaration(declaration) => match view.get(*declaration) {
            dir::Declaration::Type(_)
            | dir::Declaration::Struct(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_) => ModuleOrder::Type,
            dir::Declaration::Extension(_) => ModuleOrder::Extension,
            dir::Declaration::Function(_) => ModuleOrder::Function,
            dir::Declaration::Global(_) => ModuleOrder::Global,
            dir::Declaration::Module(_) => ModuleOrder::Module,
        },
        dir::Expression::Export { .. } => ModuleOrder::Export,
        _ => ModuleOrder::Executed,
    };

    (order, node)
}

/// Report members that move before an earlier member category.
fn report_member_order(
    module: &DirModule<'_>,
    lint: &Lint,
    members: impl IntoIterator<Item = Option<(dir::LocalNodeIdAny, MemberOrder)>>,
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    let mut furthest: Option<MemberOrder> = None;

    // compare each member with the furthest preceding category
    for member in members.into_iter().flatten() {
        let (node, order) = member;
        let previous = furthest;
        furthest = Some(furthest.map_or(order, |furthest| furthest.max(order)));
        let Some(previous) = previous else {
            continue;
        };
        if order >= previous {
            continue;
        }

        // identify the member that moves behind an earlier category
        let span = module.main_span(node)?;
        let message = format!(
            "{} must appear before an earlier {}",
            order.noun(),
            previous.noun()
        );
        output.report(lint.diagnostic(message, span));
    }

    Ok(())
}

/// Return the canonical position of one nominal declaration member.
fn member_order(member: &dir::Member) -> Option<MemberOrder> {
    let order = match member {
        dir::Member::AssociatedType { .. } => MemberOrder::AssociatedType,
        dir::Member::AssociatedConst { .. } | dir::Member::ConstBlock { .. } => {
            MemberOrder::AssociatedConstant
        }
        dir::Member::Field {
            is_static: true, ..
        }
        | dir::Member::StaticBlock { .. } => MemberOrder::StaticState,
        dir::Member::Field { .. } => MemberOrder::InstanceField,
        dir::Member::Method { signature, .. } if signature.is_constructor() => {
            MemberOrder::Constructor
        }
        dir::Member::Method {
            signature,
            is_static: true,
            ..
        } if matches!(
            signature.role,
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
        ) =>
        {
            MemberOrder::StaticAccessor
        }
        dir::Member::Method { signature, .. }
            if matches!(
                signature.role,
                Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
            ) =>
        {
            MemberOrder::InstanceAccessor
        }
        dir::Member::Method {
            is_static: true, ..
        } => MemberOrder::StaticMethod,
        dir::Member::Method { .. } => MemberOrder::InstanceMethod,
        dir::Member::Error => return None,
    };

    Some(order)
}

/// Return the canonical position of one structural type member.
fn type_member_order(member: &dir::TypeMember) -> Option<MemberOrder> {
    let order = match member {
        dir::TypeMember::AssociatedType { .. } => MemberOrder::AssociatedType,
        dir::TypeMember::AssociatedConst { .. } => MemberOrder::AssociatedConstant,
        dir::TypeMember::Field {
            is_static: true, ..
        } => MemberOrder::StaticState,
        dir::TypeMember::Field { .. } | dir::TypeMember::IndexSignature { .. } => {
            MemberOrder::InstanceField
        }
        dir::TypeMember::ConstructSignature { .. } => MemberOrder::Constructor,
        dir::TypeMember::Method {
            signature,
            is_static: true,
            ..
        } if matches!(
            signature.role,
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
        ) =>
        {
            MemberOrder::StaticAccessor
        }
        dir::TypeMember::Method { signature, .. }
            if matches!(
                signature.role,
                Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
            ) =>
        {
            MemberOrder::InstanceAccessor
        }
        dir::TypeMember::Method {
            is_static: true, ..
        } => MemberOrder::StaticMethod,
        dir::TypeMember::Method { .. } | dir::TypeMember::CallSignature { .. } => {
            MemberOrder::InstanceMethod
        }
        dir::TypeMember::Error => return None,
    };

    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report each module item that moves before an earlier category.
    #[test]
    fn test_reports_module_order() {
        let session = TestSession::dir(
            &DECLARATION_ORDER,
            r#"
function run(): void {}
struct State {}
const RETRY_LIMIT = 3;
"#,
        );

        session.assert_diagnostics(
            r#"
warning[declaration-order]: type must appear before an earlier function
 ──▶ main.ds:2:8
  │
1 │ function run(): void {}
2 │ struct State {}
  │        ^^^^^
3 │ const RETRY_LIMIT = 3;
  │

warning[declaration-order]: constant must appear before an earlier function
 ──▶ main.ds:3:1
  │
1 │ function run(): void {}
2 │ struct State {}
3 │ const RETRY_LIMIT = 3;
  │ ^^^^^
  │
"#,
        );
    }

    /// Report a field placed after an instance method.
    #[test]
    fn test_reports_member_order() {
        let session = TestSession::dir(
            &DECLARATION_ORDER,
            r#"
class State {
    reset(): void {}
    value: int32 = 0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[declaration-order]: instance field must appear before an earlier instance method
 ──▶ main.ds:3:5
  │
1 │ class State {
2 │     reset(): void {}
3 │     value: int32 = 0;
  │     ^^^^^
4 │ }
  │
"#,
        );
    }

    /// Keep static accessors before instance accessors.
    #[test]
    fn test_reports_static_accessor_after_instance_accessor() {
        let session = TestSession::dir(
            &DECLARATION_ORDER,
            r#"
declare class Store {
    get value(): string;
    static get count(): int32;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[declaration-order]: static accessor must appear before an earlier instance accessor
 ──▶ main.ds:3:16
  │
1 │ declare class Store {
2 │     get value(): string;
3 │     static get count(): int32;
  │                ^^^^^
4 │ }
  │
"#,
        );
    }

    /// Report an export placed after an executed expression.
    #[test]
    fn test_reports_export_after_executed_expression() {
        let session = TestSession::dir(
            &DECLARATION_ORDER,
            r#"
/// Run the program.
function run(): void {}

run();
export { run };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[declaration-order]: export must appear before an earlier executed expression
 ──▶ main.ds:5:1
  │
3 │
4 │ run();
5 │ export { run };
  │ ^^^^^^^^^^^^^^
  │
"#,
        );
    }

    /// Accept Vitest-style suite and test registrations after module declarations.
    #[test]
    fn test_accepts_test_registrations_last() {
        let session = TestSession::dir(
            &DECLARATION_ORDER,
            r#"
import { describe, test } from "destack:test";

module {}

const EXPECTED = 1;

struct Worker {}

extension of Worker {}

function run(): int32 {
    return EXPECTED;
}

export { Worker, run };

describe("Worker", () => {
    test("runs", () => {
        run();
    });
});
"#,
        );

        session.assert_no_diagnostics();
    }
}
