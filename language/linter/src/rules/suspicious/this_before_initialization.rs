use tspp_core::{FxIndexMap, FxIndexSet};
use tspp_dir as dir;
use tspp_repository::ProviderError;
use tspp_source::StringId;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow using 'this' before every field of the constructed value initializes.
    pub THIS_BEFORE_INITIALIZATION {
        id: "this-before-initialization",
        summary: "Disallow using 'this' before every field of the constructed value initializes",
        explanation: r#"
A constructor builds its value field by field, and 'this' names a partially built value until the last field initializes.
Instead, calls and escapes through 'this' SHOULD wait until every field holds a value.
"#,
        example: {
            reported: r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64, y: float64) {
        this.x = x;
        this.describe();
        this.y = y;
    }

    describe(&readonly this): void {}
}
"#,
            accepted: r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64, y: float64) {
        this.x = x;
        this.y = y;
        this.describe();
    }

    describe(&readonly this): void {}
}
"#,
        },
        provenance: [],
        category: Suspicious,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report 'this' uses in constructors before every field initializes.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // index the member accesses reading directly through 'this'
    let mut receiver_reads = FxIndexMap::default();
    for (_, expression) in view.iter_nodes::<dir::Expression>() {
        if let dir::Expression::Member {
            left,
            name: Some(name),
            ..
        } = expression
            && matches!(view.get(*left), dir::Expression::This)
        {
            receiver_reads.insert(*left, *name);
        }
    }

    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        let members = match declaration {
            dir::Declaration::Class(declaration) => &declaration.members,
            dir::Declaration::Struct(declaration) => &declaration.members,
            _ => continue,
        };

        // collect the instance fields and the subset a constructor must fill
        let mut fields = FxIndexSet::default();
        let mut required = FxIndexSet::default();
        for member in members {
            let dir::Member::Field {
                name: dir::Name::Identifier(name) | dir::Name::String(name),
                default,
                is_static: false,
                is_ambient: false,
                is_abstract: false,
                is_accessor: false,
                ..
            } = view.get(*member)
            else {
                continue;
            };
            fields.insert(*name);
            if default.is_none() {
                required.insert(*name);
            }
        }
        if required.is_empty() {
            continue;
        }

        for member in members {
            let dir::Member::Method {
                signature,
                body: Some(body),
                is_static: false,
                ..
            } = view.get(*member)
            else {
                continue;
            };
            if signature.role != Some(dir::FunctionRole::Constructor) {
                continue;
            }
            check_constructor(
                module,
                lint,
                &mut output,
                *body,
                &fields,
                required.clone(),
                &receiver_reads,
            )?;
        }
    }

    Ok(output)
}

/// Walk one constructor body in statement order, retiring fields as they initialize.
fn check_constructor(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
    body: dir::LocalNodeId<dir::Expression>,
    fields: &FxIndexSet<StringId>,
    mut pending: FxIndexSet<StringId>,
    receiver_reads: &FxIndexMap<dir::LocalNodeId<dir::Expression>, StringId>,
) -> Result<(), ProviderError> {
    let view = module.view();
    let dir::Expression::Block(block) = view.get(body) else {
        return Ok(());
    };
    let block = view.get(*block);
    let statements = block
        .leading_expressions
        .iter()
        .copied()
        .chain(block.tail_expression);

    for statement in statements {
        if pending.is_empty() {
            return Ok(());
        }

        // a plain assignment through 'this' initializes its field
        if let dir::Expression::Assign {
            left,
            operator: dir::AssignOperator::Assign,
            right,
        } = view.get(statement)
            && let dir::AssignPattern::Place { expression } = view.get(*left)
            && let dir::Expression::Member {
                left: receiver,
                name: Some(name),
                ..
            } = view.get(*expression)
            && matches!(view.get(*receiver), dir::Expression::This)
        {
            report_receiver_uses(
                module,
                lint,
                output,
                *right,
                fields,
                &pending,
                receiver_reads,
            )?;
            pending.swap_remove(name);
            continue;
        }

        report_receiver_uses(
            module,
            lint,
            output,
            statement,
            fields,
            &pending,
            receiver_reads,
        )?;
    }

    Ok(())
}

/// Report each 'this' inside the expression, permitting reads of initialized fields.
fn report_receiver_uses(
    module: &DirModule<'_>,
    lint: &Lint,
    output: &mut LintOutput,
    root: dir::LocalNodeId<dir::Expression>,
    fields: &FxIndexSet<StringId>,
    pending: &FxIndexSet<StringId>,
    receiver_reads: &FxIndexMap<dir::LocalNodeId<dir::Expression>, StringId>,
) -> Result<(), ProviderError> {
    let view = module.view();
    let extent = module.source_extent(root.into_any())?;

    for (node, expression) in view.iter_nodes::<dir::Expression>() {
        if !matches!(expression, dir::Expression::This) {
            continue;
        }
        let span = module.main_span(node.into_any())?;
        if span.file != extent.file || span.start < extent.start || span.end > extent.end {
            continue;
        }
        // a read of an initialized field observes a completed part
        if let Some(name) = receiver_reads.get(&node)
            && fields.contains(name)
            && !pending.contains(name)
        {
            continue;
        }
        output.report(lint.diagnostic("'this' is used before every field initializes", span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::THIS_BEFORE_INITIALIZATION;
    use crate::tests::TestSession;

    /// Report a method call through 'this' before the last field initializes.
    #[test]
    fn test_reports_call_before_last_field() {
        let session = TestSession::dir(
            &THIS_BEFORE_INITIALIZATION,
            r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64, y: float64) {
        this.x = x;
        this.describe();
        this.y = y;
    }

    describe(&readonly this): void {}
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[this-before-initialization]: 'this' is used before every field initializes
 ──▶ main.tspp:7:9
  │
5 │     constructor(x: float64, y: float64) {
6 │         this.x = x;
7 │         this.describe();
  │         ^^^^
8 │         this.y = y;
9 │     }
  │
"#,
        );
    }

    /// Report an escaping 'this' before any field initializes.
    #[test]
    fn test_reports_escape_before_first_field() {
        let session = TestSession::dir(
            &THIS_BEFORE_INITIALIZATION,
            r#"
function register(point: Point): void {}

class Point {
    x: float64;

    constructor(x: float64) {
        register(this);
        this.x = x;
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[this-before-initialization]: 'this' is used before every field initializes
 ──▶ main.tspp:7:18
  │
5 │
6 │     constructor(x: float64) {
7 │         register(this);
  │                  ^^^^
8 │         this.x = x;
9 │     }
  │
"#,
        );
    }

    /// Accept 'this' uses after every field initializes.
    #[test]
    fn test_accepts_use_after_all_fields() {
        let session = TestSession::dir(
            &THIS_BEFORE_INITIALIZATION,
            r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64, y: float64) {
        this.x = x;
        this.y = y;
        this.describe();
    }

    describe(&readonly this): void {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a read of an already initialized field between assignments.
    #[test]
    fn test_accepts_read_of_initialized_field() {
        let session = TestSession::dir(
            &THIS_BEFORE_INITIALIZATION,
            r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64) {
        this.x = x;
        this.y = this.x * 2.0;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept constructor uses when every remaining field carries a default.
    #[test]
    fn test_accepts_use_with_defaulted_fields() {
        let session = TestSession::dir(
            &THIS_BEFORE_INITIALIZATION,
            r#"
class Counter {
    count: int32 = 0;

    constructor() {
        this.bump();
    }

    bump(): void {}
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
