import { escapeAttribute } from "./html";

/// Public rule attributes displayed in the reference header.
export type RuleMetadata = {
    category: string;
    level: "error" | "warning";
    fixability: "automatic" | "suggestion" | "none";
    scope: "module" | "program";
};

/// An upstream rule credited by the linter.
export type RuleProvenance = { source: string; rule: string; url: string; };

/// The implementation's repository location.
export type RuleSource = { path: string; line: number; };

/// Render the stable lint attributes.
export function renderLintMetadata(rule: RuleMetadata): string {
    const labels = [
        ["Category", rule.category.charAt(0).toUpperCase() + rule.category.slice(1)],
        ["Severity", rule.level === "error" ? "Error" : "Warning"],
        ["Fix", { automatic: "Automatic", suggestion: "Suggested", none: "None" }[rule.fixability]],
        ["Scope", rule.scope === "module" ? "Module" : "Program"],
    ];

    return `<dl class="lint-metadata">${labels.map(([label, value]) =>
        `<div><dt>${label}</dt><dd>${escapeAttribute(value)}</dd></div>`
    ).join("")}</dl>`;
}

/// Render upstream rules and implementation as named reference entries.
export function renderLintReferences(provenance: readonly RuleProvenance[], source: RuleSource): string {
    const route = `https://github.com/destack-sh/destack/blob/main/${source.path}#L${source.line}`;
    const entries = [
        ...provenance.map((entry) => ({ label: "Inspired by", title: entry.rule, href: entry.url })),
        { label: "Source", title: `${source.path.split("/").at(-1)}:${source.line}`, href: route },
    ];

    // provenance and source links share one scale, alignment, and interaction
    return `<dl class="reference-links">${entries.map((entry) =>
        `<div><dt>${entry.label}</dt><dd><a href="${escapeAttribute(entry.href)}" rel="external noopener noreferrer" target="_blank">${escapeAttribute(entry.title)} <span aria-hidden="true">↗</span></a></dd></div>`
    ).join("")}</dl>`;
}
