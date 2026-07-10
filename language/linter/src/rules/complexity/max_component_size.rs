use std::collections::{HashMap, HashSet};

use destack_artifact::ComponentGraph;
use destack_repository::LintSeverity;
use destack_source::{ModuleId, Span};

use crate::rules::common::find_cycle_path;
use crate::{LintMeta, LintPackageContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of modules in one dependency-cycle component.
    ///
    /// Modules in an import cycle check as one unit: they cannot check in
    /// parallel, and editing any member re-checks the whole component.
    /// Consider breaking the cycle at one of the reported edges.
    #[lint(
        id = "max-component-size",
        code = "LX027",
        category = Complexity,
        level = Dir,
        scope = Package,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub MaxComponentSize,
    "Limit the number of modules in one dependency-cycle component"
}

impl LintRule for MaxComponentSize {
    fn meta(&self) -> &'static LintMeta {
        MaxComponentSize::meta()
    }

    fn check_package(&self, ctx: &mut LintPackageContext) {
        // resolve lint metadata and the size threshold
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }
        let max_members = ctx.options().complexity.max_component_size;

        // read the checked component partition
        let Some(graph) = ctx.session.component_graph() else {
            return;
        };

        // group the package's modules by their component
        let mut members_by_component = HashMap::new();
        for module_id in ctx.package_module_ids() {
            let Some(component) = graph.component(module_id) else {
                continue;
            };
            members_by_component
                .entry(component)
                .or_insert_with(Vec::new)
                .push(module_id);
        }

        // report each oversized component once, on its first package member
        for (component, mut package_members) in members_by_component {
            let members = graph.members(component);
            if members.len() <= max_members {
                continue;
            }
            package_members.sort_unstable();
            let Some(anchor) = package_members.first().copied() else {
                continue;
            };

            let diagnostic = build_component_diagnostic(
                anchor,
                members,
                &graph,
                max_members,
                severity,
                meta.id,
                ctx,
            );
            ctx.report(diagnostic);
        }
    }
}

/// Build one oversized component diagnostic.
fn build_component_diagnostic(
    anchor: ModuleId,
    members: &[ModuleId],
    graph: &ComponentGraph,
    max_members: usize,
    severity: LintSeverity,
    rule_id: &str,
    ctx: &LintPackageContext,
) -> LintReport {
    let module = ctx
        .session
        .repository_module(anchor)
        .unwrap_or_else(|| panic!("missing module for {anchor:?}"));
    let module = module.as_ref();

    // name a bounded member sample
    let sample = members
        .iter()
        .take(8)
        .map(|member| member_name(ctx, *member))
        .collect::<Vec<_>>()
        .join(", ");
    let sample = match members.len() > 8 {
        true => format!("{sample}, …"),
        false => sample,
    };

    let mut diagnostic = LintReport::new(
        MAX_COMPONENT_SIZE.id,
        MAX_COMPONENT_SIZE.code,
        MAX_COMPONENT_SIZE.category,
        severity,
        format!(
            "dependency cycle spans {} modules, more than the {max_members} allowed",
            members.len()
        ),
        Span::empty(module.file_id),
    )
    .label("this module checks as one unit with every cycle member")
    .note(format!("cycle members: {sample}"))
    .note(format!("rule: {rule_id}"));

    // show one concrete cycle to cut
    if let Some(cycle_note) = cycle_note(ctx, graph, members) {
        diagnostic = diagnostic.note(cycle_note);
    }

    diagnostic
}

/// Return one component member's display name.
fn member_name(ctx: &LintPackageContext, member: ModuleId) -> String {
    ctx.session
        .repository_module(member)
        .and_then(|module| ctx.session.repository_file(module.file_id))
        .map(|file| file.name.clone())
        .unwrap_or_else(|| member.to_string())
}

/// Build one example cycle note through the component.
fn cycle_note(
    ctx: &LintPackageContext,
    graph: &ComponentGraph,
    members: &[ModuleId],
) -> Option<String> {
    // restrict the component's edges to its own members
    let member_set = members.iter().copied().collect::<HashSet<_>>();
    let mut adjacency = HashMap::new();
    for member in members {
        let edges = graph
            .edges(*member)
            .iter()
            .copied()
            .filter(|target| member_set.contains(target))
            .collect::<Vec<_>>();
        adjacency.insert(*member, edges);
    }

    let cycle = find_cycle_path(&adjacency, members)?;
    if cycle.len() < 2 {
        return None;
    }

    let names = cycle
        .iter()
        .map(|member| member_name(ctx, *member))
        .collect::<Vec<_>>();

    Some(format!("example cycle: {}", names.join(" -> ")))
}
