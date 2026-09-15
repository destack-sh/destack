use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::{DiagnosticSuggestion, Patch};

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require readonly for private fields never mutated after initialization.
    pub PREFER_READONLY {
        id: "prefer-readonly",
        summary: "Require readonly for private fields never mutated after initialization",
        explanation: r#"
A private field that is never mutated after initialization grants unnecessary write access inside its declaration.
Instead, you SHOULD mark the field `readonly` so later mutation is rejected.
"#,
        example: {
            reported: r#"
class User {
    private name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(): string {
        return this.name;
    }
}
"#,
            accepted: r#"
class User {
    private readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    describe(): string {
        return this.name;
    }
}
"#,
        },
        provenance: [TypeScriptEslint("prefer-readonly")],
        category: Style,
        level: Warning,
        fixable: Automatic,
        check: DirModule(check),
    }
}

/// Report private fields whose uses are readonly after initialization.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let occurrences = module.flows.access_occurrences().collect::<Vec<_>>();
    let mut unfinished_owners = FxIndexSet::default();
    let mut output = LintOutput::default();

    // collect nominal owners with wholly unfinished methods
    for (member, node) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            body: Some(body), ..
        } = node
        else {
            continue;
        };
        let Some(owner) = module.member_owner(member)? else {
            continue;
        };
        let Some(expression) = module.sole_expression(*body) else {
            continue;
        };
        if !matches!(view.get(expression), dir::Expression::Call { .. }) {
            continue;
        }
        if module.language_item(expression)? != Some(dir::LanguageItem::Todo) {
            continue;
        }
        unfinished_owners.insert(owner);
    }

    // inspect concrete nominal declarations and their constructor bodies
    for (declaration, node) in view.iter_nodes::<dir::Declaration>() {
        let members = match node {
            dir::Declaration::Class(declaration) if !declaration.is_ambient => &declaration.members,
            dir::Declaration::Struct(declaration) if !declaration.is_ambient => {
                &declaration.members
            }
            _ => continue,
        };
        let owner = module.declaration_symbol(declaration)?;
        if unfinished_owners.contains(&owner) {
            continue;
        }
        let constructors = members
            .iter()
            .filter_map(|member| match view.get(*member) {
                dir::Member::Method {
                    signature,
                    body: Some(body),
                    ..
                } if signature.role == Some(dir::FunctionRole::Constructor) => Some(*body),
                _ => None,
            })
            .collect::<Vec<_>>();

        // inspect private mutable fields
        for member in members.iter().copied() {
            let dir::Member::Field {
                name,
                visibility: Some(dir::Visibility::Private),
                is_readonly: false,
                is_ambient: false,
                is_abstract: false,
                is_static,
                is_accessor: false,
                ..
            } = view.get(member)
            else {
                continue;
            };
            let key = (*name).into();

            // reject mutable uses outside the nearest constructor body
            let mut is_mutated = false;
            for occurrence in &occurrences {
                if !occurrence.uses.may_mutate() || occurrence.path.keys() != [key] {
                    continue;
                }

                // retain uses in the declaration and extensions of the same owner
                let is_inside = view.is_inside(occurrence.node, declaration.into_any());
                let is_static_owner = occurrence.path.root() == dir::AccessRoot::Symbol(owner);
                let is_owned = if is_inside || is_static_owner {
                    true
                } else if occurrence.path.root() == dir::AccessRoot::Receiver {
                    let Some(member) = view.ancestor::<dir::Member>(occurrence.node) else {
                        continue;
                    };

                    module.member_owner(member)? == Some(owner)
                } else {
                    false
                };
                if !is_owned {
                    continue;
                }

                // permit direct receiver initialization only in the constructor body
                let callable = module.enclosing_callable_body(occurrence.node);
                let is_direct_receiver = occurrence.path.root() == dir::AccessRoot::Receiver
                    && occurrence.path.keys() == [key];
                let is_initialization = !*is_static
                    && is_direct_receiver
                    && callable.is_some_and(|body| constructors.contains(&body));
                if !is_initialization {
                    is_mutated = true;
                    break;
                }
            }

            if is_mutated {
                continue;
            }

            // insert readonly before the authored field name
            let span = module.main_span(member.into_any())?;
            let diagnostic = lint
                .diagnostic("private field is never mutated after initialization", span)
                .suggestion(suggestion(lint, span)?);
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Build one readonly field annotation.
fn suggestion(
    lint: &Lint,
    name: destack_source::Span,
) -> Result<DiagnosticSuggestion, ProviderError> {
    let patch = Patch::insert(name.file, name.start, "readonly ");

    lint.fix("mark the field readonly", patch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Mark a private struct field without mutable uses readonly.
    #[test]
    fn test_marks_struct_field_readonly() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
struct Point {
    private x: int32;

    getX(): int32 {
        return this.x;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
struct Point {
    private readonly x: int32;

    getX(): int32 {
        return this.x;
    }
}
"#,
        );
    }

    /// Mark a private field initialized at its declaration readonly.
    #[test]
    fn test_marks_initialized_field_readonly() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;

    read(): int32 {
        return this.value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
class Counter {
    private readonly value: int32 = 0;

    read(): int32 {
        return this.value;
    }
}
"#,
        );
    }

    /// Mark a private static field without mutable uses readonly.
    #[test]
    fn test_marks_static_field_readonly() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private static value: int32 = 0;

    static read(): int32 {
        return Counter.value;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
class Counter {
    private static readonly value: int32 = 0;

    static read(): int32 {
        return Counter.value;
    }
}
"#,
        );
    }

    /// Accept a field written outside its constructor.
    #[test]
    fn test_accepts_mutated_field() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;

    increment(): void {
        this.value += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Treat a constructor write to a static field as ordinary mutation.
    #[test]
    fn test_accepts_static_field_mutated_by_constructor() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private static value: int32 = 0;

    constructor() {
        Counter.value += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept public fields whose external uses are not visible.
    #[test]
    fn test_accepts_public_field() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class User {
    name: string = "";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Treat a nested constructor closure as ordinary post-construction code.
    #[test]
    fn test_accepts_nested_constructor_write() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;

    constructor() {
        const increment = () => {
            this.value += 1;
        };
        increment();
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a private field mutated through another instance.
    #[test]
    fn test_accepts_other_instance_write() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;

    copyTo(other: Counter): void {
        other.value = this.value;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Treat a constructor write through another instance as ordinary mutation.
    #[test]
    fn test_accepts_other_instance_constructor_write() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;

    constructor(other: Counter) {
        other.value = 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Mark a field readonly when only storage beneath its value is mutated.
    #[test]
    fn test_marks_field_with_nested_mutation_readonly() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class State {
    value: int32 = 0;
}

class Counter {
    private state: State;

    constructor(state: State) {
        this.state = state;
    }

    increment(): void {
        this.state.value += 1;
    }
}
"#,
        );

        session.assert_fixes(
            r#"
class State {
    value: int32 = 0;
}

class Counter {
    private readonly state: State;

    constructor(state: State) {
        this.state = state;
    }

    increment(): void {
        this.state.value += 1;
    }
}
"#,
        );
    }

    /// Accept private fields mutated by inherent extension methods.
    #[test]
    fn test_accepts_extension_mutation() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
class Counter {
    private value: int32 = 0;
}

extension of Counter {
    increment(): void {
        this.value += 1;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve private field mutability while an owner method remains unfinished.
    #[test]
    fn test_accepts_unfinished_owner() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
import { todo } from "destack:error";

class Counter {
    private value: int32 = 0;
}

extension of Counter {
    increment(): void {
        todo("Counter.increment");
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an immutable subclass whose constructor delegates to its superclass.
    #[test]
    fn test_accepts_super_constructor() {
        let session = TestSession::dir(
            &PREFER_READONLY,
            r#"
import { ContextVar } from "destack:context";
import { Dynamic, DynamicSafe } from "destack:memory";

/// One dynamically scoped binding family.
export local class Binding<T: DynamicSafe> extends ContextVar<Dynamic<T>> {
    /// Create one binding family resolved through the seeded context.
    constructor(name: string) {
        super(name);
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
