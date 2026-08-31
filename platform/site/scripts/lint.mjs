import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { highlightCodeFragments } from "./highlight.mjs";
import { renderMarkdown } from "./markdown.mjs";
import { plainTextFor, tokenEstimateFor } from "./text.mjs";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const lintReferenceFile = join(
    repositoryDirectory,
    "platform/site/.generated/lint-reference.json",
);
const ruleCatalogPath = "language/static-analysis/linter/rules";
const ruleCatalogRoute = `/docs/${ruleCatalogPath}/`;
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

    const reference = JSON.parse(readFileSync(lintReferenceFile, "utf8"));
    if (reference.schemaVersion !== 1 || !Array.isArray(reference.rules)) {
        throw new Error(`unsupported lint reference in ${lintReferenceFile}`);
    }

    return reference;
}

/// Render the lint catalog and every registered rule.
export function renderLintDocuments(reference, index) {
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
function renderLintCatalog(reference, index) {
    const knownCategories = new Set(lintCategories.map(([category]) => category));
    const unknown = reference.rules.find((rule) => !knownCategories.has(rule.category));
    if (unknown != undefined) {
        throw new Error(`unknown lint category: ${unknown.category}`);
    }

    const categories = lintCategories.map(([category, title]) => {
        const rules = reference.rules.filter((rule) => rule.category === category);
        const items = rules.map((rule) => {
            const route = lintRuleRoute(rule.id);

            const summary = escapeHtml(rule.summary);

            return `<li><a href="${route}"><code>${escapeHtml(rule.id)}</code><span title="${escapeAttribute(rule.summary)}">${summary}</span></a></li>`;
        }).join("");
        const label = rules.length === 1 ? "1 rule" : `${rules.length} rules`;
        const html = `<section class="lint-category"><h2 id="${category}"><span>${title}</span><span>${label}</span></h2><ul class="lint-rule-list">${items}</ul></section>`;
        const markdown = `## ${title}\n\n${rules.map((rule) =>
            `- [${rule.id}](${lintRuleRoute(rule.id)}) — ${rule.summary}`
        ).join("\n")}`;

        return { category, html, markdown, rules, title };
    });
    const markdown = `${index.markdown}\n\n${categories.map((category) => category.markdown).join("\n\n")}`;
    const html = `${index.html}${categories.map((category) => category.html).join("")}`;
    const searchSections = [
        ...index.searchSections,
        ...categories.map(({ category, rules, title }) => ({
            depth: 2,
            id: category,
            text: rules.map((rule) => `${rule.id} ${rule.summary}`).join(" "),
            title,
        })),
    ];
    const tableOfContents = categories.map(({ category, title }) => ({
        depth: 2,
        id: category,
        text: title,
    }));

    return {
        ...index,
        html,
        markdown,
        searchSections,
        searchText: searchSections.map((section) => section.text).join(" "),
        tableOfContents,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Render one lint rule page.
function renderLintRule(rule, reported, accepted, order) {
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
    const html = `<div class="lint-rule">${metadata}${explanation}<section class="lint-examples">${examples}</section>${references}</div>`;
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
        lead: summary,
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

/// Render the stable lint attributes.
function renderLintMetadata(rule) {
    const fixability = {
        automatic: "automatic fix",
        none: "no fix",
        suggestion: "suggested fix",
    }[rule.fixability];
    if (fixability == undefined) {
        throw new Error(`unknown lint fixability: ${rule.fixability}`);
    }

    const tags = [
        ["category", rule.category, rule.category],
        ["level", rule.level, rule.level],
        ["fix", fixability, rule.fixability],
        ["scope", rule.scope, rule.scope],
    ];
    const items = tags.map(([kind, text, value]) =>
        `<span class="lint-tag lint-tag-${escapeAttribute(kind)} lint-tag-${escapeAttribute(value)}">${escapeHtml(text)}</span>`
    ).join("");

    return `<div class="lint-metadata">${items}</div>`;
}

/// Render one canonical lint example.
function renderLintExample(title, highlighted) {
    const lines = highlighted.split("\n").map((line, index) => {
        const text = line === "" ? " " : line;

        return `<span class="markdown-code-line" data-publication-line><span class="markdown-code-gutter" data-publication-gutter>${index + 1}</span><span class="markdown-code-text" data-publication-code>${text}</span></span>`;
    }).join("");

    return `<section class="lint-example"><h2>${escapeHtml(title)}</h2><figure class="markdown-code" data-publication-listing><pre data-publication-body tabindex="0"><code class="markdown-code-lines" data-publication-lines>${lines}</code></pre></figure></section>`;
}

/// Render prior art and the lint declaration as one reference row.
function renderLintReferences(provenance, source) {
    const route = `https://github.com/destack-sh/destack/blob/main/${source.path}#L${source.line}`;
    const links = provenance.map((entry) =>
        `<a href="${escapeAttribute(entry.url)}" rel="external noopener noreferrer" target="_blank">${escapeHtml(entry.source)} · ${escapeHtml(entry.rule)}</a>`
    ).join("");

    return `<footer class="lint-references">${links}<a href="${escapeAttribute(route)}" rel="external noopener noreferrer" target="_blank" title="${escapeAttribute(`${source.path}:${source.line}`)}">source</a></footer>`;
}

/// Render lint Markdown through the normal content renderer.
function renderLintMarkdown(markdown, route) {
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
function renderLintMarkdownSource(rule) {
    const provenance = rule.provenance.length === 0
        ? ""
        : `\n\n## Prior art\n\n${rule.provenance.map((entry) =>
            `- [${entry.source} · ${entry.rule}](${entry.url})`
        ).join("\n")}`;
    const sourceRoute = `https://github.com/destack-sh/destack/blob/main/${rule.source.path}#L${rule.source.line}`;
    const sourceLabel = `${rule.source.path}:${rule.source.line}`;

    return `# ${rule.id}\n\n${rule.summary}\n\n${rule.explanation}\n\n- Category: ${rule.category}\n- Level: ${rule.level}\n- Fix: ${rule.fixability}\n- Scope: ${rule.scope}\n\n## Reported\n\n\`\`\`ds title="${rule.example.reported.path}"\n${rule.example.reported.source}\n\`\`\`\n\n## Accepted\n\n\`\`\`ds title="${rule.example.accepted.path}"\n${rule.example.accepted.source}\n\`\`\`${provenance}\n\n[${sourceLabel}](${sourceRoute})\n`;
}

/// Return the canonical route for one lint rule.
function lintRuleRoute(id) {
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(id)) {
        throw new Error(`invalid lint id: ${id}`);
    }

    return `${ruleCatalogRoute}${id}/`;
}

/// Return the generated portable source path for one lint rule.
function lintRulePath(id) {
    const route = lintRuleRoute(id).slice("/docs/".length);

    return `${route}index.md`;
}

/// Escape text placed in generated HTML.
function escapeHtml(value) {
    return value
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;");
}

/// Escape text placed in generated HTML attributes.
function escapeAttribute(value) {
    return escapeHtml(value).replaceAll("'", "&#39;");
}
