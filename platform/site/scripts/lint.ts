import { escapeHtml, escapeAttribute } from "../src/content/html.ts";
import type { DocumentationPage } from "./page";
import { renderLintMetadata, renderLintReferences, type RuleMetadata, type RuleProvenance, type RuleSource } from "../src/content/lint.ts";
import { renderListing } from "../src/content/listing.ts";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { highlightCodeFragments } from "./highlight.ts";
import { renderMarkdown } from "./markdown.ts";
import { plainTextFor, tokenEstimateFor } from "./text.ts";

/// One checked rule emitted by the linter CLI.
type LintRule = RuleMetadata & {
    /// The stable rule identifier.
    id: string;
    /// The short rule description.
    summary: string;
    /// The authored rule explanation.
    explanation: string;
    /// The source examples before and after correction.
    example: {
        reported: { source: string; path: string };
        accepted: { source: string; path: string };
    };
    /// The upstream rules credited by this rule.
    provenance: RuleProvenance[];
    /// The implementation location.
    source: RuleSource;
};

/// The versioned CLI rule inventory.
type LintReference = {
    schemaVersion: number;
    rules: LintRule[];
};

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const lintReferenceFile = join(
    repositoryDirectory,
    "platform/site/.generated/lint-reference.json",
);
const ruleCatalogPath = "language/static-analysis/linter";
/// The linter rule catalogue.
export const ruleCatalogRoute = `/docs/${ruleCatalogPath}/`;
const lintCategories = [
    ["correctness", "Correctness"],
    ["suspicious", "Suspicious"],
    ["security", "Security"],
    ["performance", "Performance"],
    ["style", "Style"],
];

/// Read the generated lint reference.
export function readLintReference() {
    if (!existsSync(lintReferenceFile)) {
        throw new Error("missing generated lint reference, run `just platform/site/generate`");
    }

    const reference: LintReference = JSON.parse(readFileSync(lintReferenceFile, "utf8"));
    if (reference.schemaVersion !== 1 || !Array.isArray(reference.rules)) {
        throw new Error(`unsupported lint reference in ${lintReferenceFile}`);
    }

    return reference;
}

/// Render the lint catalog and every registered rule.
export function renderLintDocuments(reference: LintReference, index: DocumentationPage) {
    const highlights = highlightCodeFragments(
        reference.rules.flatMap((rule) => [
            rule.example.reported.source,
            rule.example.accepted.source,
        ]),
        "ds",
    );
    let highlightIndex = 0;
    const items = reference.rules.map((rule) => {
        const reported = highlights[highlightIndex];
        const accepted = highlights[highlightIndex + 1];
        highlightIndex += 2;

        return renderLintRule(rule, reported, accepted, index.order);
    });
    const catalog = renderLintCatalog(reference, index);

    return { documents: [catalog], items };
}

/// Render the flat lint rule catalog.
function renderLintCatalog(reference: LintReference, index: DocumentationPage) {
    const knownCategories = new Set(lintCategories.map(([category]) => category));
    const unknown = reference.rules.find((rule) => !knownCategories.has(rule.category));
    if (unknown != undefined) {
        throw new Error(`unknown lint category: ${unknown.category}`);
    }

    // align filtering and metadata with the same columns in the static reference
    const rules = [...reference.rules].sort((left, right) => left.id.localeCompare(right.id));
    const select = (name: string, title: string, options: string[][]) => `<select name="${name}" aria-label="${title}" disabled><option value="">${title}</option>${options.map(([value, label]) => `<option value="${value}">${label}</option>`).join("")}</select>`;
    const category = select("category", "Category", lintCategories);
    const level = select("level", "Severity", [["error", "Error"], ["warning", "Warning"]]);
    const fix = select("fixability", "Fix", [["automatic", "Automatic"], ["suggestion", "Suggested"], ["none", "No fix"]]);
    const scope = select("scope", "All scopes", [["module", "Module"], ["program", "Program"]]);
    const items = rules.map((rule) => {
        const search = [rule.id, rule.summary, rule.category, rule.level, rule.scope, ...rule.provenance.map((entry) => `${entry.source} ${entry.rule}`)].join(" ").toLowerCase();
        const attributes = (["category", "level", "fixability", "scope"] as const).map((key) => `data-${key}="${escapeAttribute(rule[key])}"`).join(" ");
        const fixLabel = { automatic: "Automatic", suggestion: "Suggested", none: "—" }[rule.fixability];

        return `<tr data-rule data-search="${escapeAttribute(search)}" ${attributes}><td class="lint-name"><a href="${lintRuleRoute(rule.id)}"><code>${escapeHtml(rule.id)}</code></a><p>${escapeHtml(rule.summary)}</p></td><td data-label="Category"><span class="content-tag">${escapeHtml(rule.category)}</span></td><td data-label="Severity"><span class="lint-severity lint-severity-${rule.level}">${escapeHtml(rule.level)}</span></td><td data-label="Fix support"><span class="lint-fix" title="${rule.fixability === "none" ? "No fix available" : `${fixLabel} fix`}">${fixLabel}</span></td></tr>`;
    }).join("");
    const html = `${index.html}<section id="rules" class="lint-catalog" aria-label="Linter rules"><div class="lint-controls"><div class="lint-toolbar" hidden><label class="lint-search"><svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 4.5 4.5"/></svg><input type="search" aria-label="Search rules" name="q" placeholder="Find a rule…" autocomplete="off" /></label>${scope}<span class="lint-count" role="status" aria-live="polite">${rules.length} rules</span><button type="button" data-clear aria-label="Clear search and filters" title="Clear search and filters">Clear</button></div><table class="lint-table"><thead><tr><th scope="col">Rule</th><th scope="col">${category}</th><th scope="col">${level}</th><th scope="col">${fix}</th></tr></thead><tbody>${items}</tbody></table></div><p class="lint-empty" hidden>No matching rules.</p></section>`;
    const markdown = `${index.markdown}\n\n## Rules\n\n${rules.map((rule) => `- [${rule.id}](${lintRuleRoute(rule.id)}) — ${rule.summary} (${rule.category}; ${rule.level}; ${rule.fixability})`).join("\n")}`;
    const searchSections = [...index.searchSections, {
        depth: 2, id: "rules", title: "Rules",
        text: rules.map((rule) => `${rule.id} ${rule.summary}`).join(" "),
    }];
    const tableOfContents: DocumentationPage["tableOfContents"] = [];

    return {
        ...index,
        lead: undefined,
        searchKind: "catalog",
        html,
        markdown,
        searchSections,
        searchText: searchSections.map((section) => section.text).join(" "),
        tableOfContents,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Render one lint rule page.
function renderLintRule(rule: LintRule, reported: string, accepted: string, order: number) {
    if (reported == undefined || accepted == undefined) {
        throw new Error(`missing highlighted lint example: ${rule.id}`);
    }

    const route = lintRuleRoute(rule.id);
    const explanation = renderLintMarkdown(rule.explanation, route);
    const metadata = renderLintMetadata(rule);
    const examples = [
        renderLintExample("Reported", reported),
        renderLintExample("Accepted", accepted),
    ].join("");
    const references = renderLintReferences(rule.provenance, rule.source);
    const html = `<div class="lint-rule">${metadata}<div class="lint-description"><p>${escapeHtml(rule.summary)}</p>${explanation}</div><section class="lint-examples">${examples}</section>${references}</div>`;
    const markdown = renderLintMarkdownSource(rule);
    const path = lintRulePath(rule.id);
    const summary = plainTextFor(rule.summary);
    const searchText = [
        rule.id,
        rule.summary,
        rule.explanation,
        ...rule.provenance.map((entry) => `${entry.source} ${entry.rule}`),
    ].join(" ");

    return {
        assets: [],
        description: summary,
        file: lintReferenceFile,
        headings: [{ depth: 1, id: rule.id, text: rule.id }],
        html,
        lead: undefined,
        markdown,
        markdownRoute: `/docs/${path}`,
        order,
        path,
        route,
        searchContext: "linter",
        searchKind: "rule",
        searchSections: [{ depth: 1, id: rule.id, text: searchText, title: rule.id }],
        searchText,
        tableOfContents: [],
        textRoute: `/docs/${path.replace(/\.md$/, ".txt")}`,
        title: rule.id,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Render one canonical lint example.
function renderLintExample(title: string, highlighted: string) {
    return `<section class="lint-example">${renderListing(highlighted, { title, label: title })}</section>`;
}

/// Render lint Markdown through the normal content renderer.
function renderLintMarkdown(markdown: string, route: string) {
    return renderMarkdown(markdown, {
        assets: [],
        documentDirectory: repositoryDirectory,
        kind: "document",
        markdownDirectory: dirname(lintReferenceFile),
        ownHeadings: new Set(),
        route,
        slug: route,
        sourceRoutes: new Map(),
    });
}

/// Render one portable Markdown lint reference.
function renderLintMarkdownSource(rule: LintRule) {
    const provenance = rule.provenance.length === 0
        ? ""
        : `\n\n## Inspired by\n\n${rule.provenance.map((entry) =>
            `- [${entry.rule}](${entry.url})`
        ).join("\n")}`;
    const sourceRoute = `https://github.com/destack-sh/destack/blob/main/${rule.source.path}#L${rule.source.line}`;
    const sourceLabel = `${rule.source.path}:${rule.source.line}`;

    return `# ${rule.id}\n\n${rule.summary}\n\n${rule.explanation}\n\n- Category: ${rule.category}\n- Level: ${rule.level}\n- Fix: ${rule.fixability}\n- Scope: ${rule.scope}\n\n## Reported\n\n\`\`\`ds title="${rule.example.reported.path}"\n${rule.example.reported.source}\n\`\`\`\n\n## Accepted\n\n\`\`\`ds title="${rule.example.accepted.path}"\n${rule.example.accepted.source}\n\`\`\`${provenance}\n\n## Source\n\n[${sourceLabel}](${sourceRoute})\n`;
}

/// Return the canonical route for one lint rule.
function lintRuleRoute(id: string) {
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id)) {
        throw new Error(`invalid lint id: ${id}`);
    }

    return `${ruleCatalogRoute}${id}/`;
}

/// Return the generated portable source path for one lint rule.
function lintRulePath(id: string) {
    const route = lintRuleRoute(id).slice("/docs/".length);

    return `${route}index.md`;
}
