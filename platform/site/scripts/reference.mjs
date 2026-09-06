import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { highlightCodeFragments } from "./highlight.mjs";
import { renderMarkdown } from "./markdown.mjs";
import { plainTextFor, tokenEstimateFor } from "./text.mjs";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const documentDirectory = join(repositoryDirectory, "language/library");
const libraryReferenceFile = join(
    repositoryDirectory,
    "platform/site/.generated/library-reference.json",
);
const moduleCatalogPath = "language/standard-library/modules";
/// The standard library module catalogue.
export const moduleCatalogRoute = `/docs/${moduleCatalogPath}/`;
const libraryKindOrder = [
    "namespace",
    "class",
    "struct",
    "enum",
    "newtype",
    "newtype_interface",
    "interface",
    "type_alias",
    "function",
    "constant",
    "extension",
];
const libraryKindTitles = new Map([
    ["namespace", "Namespaces"],
    ["class", "Classes"],
    ["struct", "Structs"],
    ["enum", "Enums"],
    ["newtype", "Newtypes"],
    ["newtype_interface", "Newtype interfaces"],
    ["interface", "Interfaces"],
    ["type_alias", "Type aliases"],
    ["function", "Functions"],
    ["constant", "Constants"],
    ["extension", "Extensions"],
]);

/// Read the standard library package reference.
export function readLibraryReference() {
    if (!existsSync(libraryReferenceFile)) {
        throw new Error("missing generated library reference, run `just platform/site/generate`");
    }

    const reference = JSON.parse(readFileSync(libraryReferenceFile, "utf8"));
    if (reference.schemaVersion !== 5 || !Array.isArray(reference.modules)) {
        throw new Error(`unsupported library reference in ${libraryReferenceFile}`);
    }

    return reference;
}

/// Render the standard library catalog, module indexes, and items.
export function renderLibraryDocuments(reference, index) {
    const order = index.order;
    const namespaceRoutes = new Map(reference.modules.map((module) => [
        libraryModuleDisplay(reference.package.name, module.path),
        libraryModuleRoute(module.specifier),
    ]));
    const moduleItems = new Map(reference.modules.map((module) => [
        module.specifier,
        libraryModuleItems(module, namespaceRoutes),
    ]));
    const canonicalItems = canonicalLibraryItems(reference.modules, moduleItems);

    // point every re-export at one canonical item page
    for (const items of moduleItems.values()) {
        for (const item of items) {
            if (item.kind === "namespace") {
                continue;
            }
            const canonical = canonicalItems.get(item.identity);
            if (canonical == undefined) {
                throw new Error(`missing canonical library item: ${item.identity}`);
            }
            item.route = libraryItemRoute(canonical.module.specifier, item.kind, item.name);
        }
    }

    const catalog = renderLibraryCatalog(reference, index);
    const modules = reference.modules.map((module) => {
        const items = moduleItems.get(module.specifier);
        if (items == undefined) {
            throw new Error(`missing indexed items for ${module.specifier}`);
        }

        return renderLibraryModule(module, items, order);
    });

    // highlight every definition with one parser process
    const canonical = [...canonicalItems.values()]
        .sort((left, right) => left.route.localeCompare(right.route));
    const references = libraryReferences(canonical);
    const definitions = canonical.flatMap((item) =>
        item.declarations.map((declaration) => libraryDefinition(declaration))
    );
    const memberSignatures = canonical.flatMap((item) =>
        item.declarations.flatMap((declaration) =>
            declaration.members.map((member) => librarySignature(member))
        )
    );
    const highlighted = highlightCodeFragments(
        [...definitions, ...memberSignatures].map((definition) => definition.text),
        "ds",
        [...definitions, ...memberSignatures].map((definition) => definition.tokens),
        references,
    );

    // assign each highlighted definition and member back to its canonical item
    let definitionIndex = 0;
    let memberIndex = definitions.length;
    const items = canonical.map((item) => {
        const declarationHighlights = highlighted.slice(
            definitionIndex,
            definitionIndex + item.declarations.length,
        );
        const memberCount = item.declarations.reduce(
            (count, declaration) => count + declaration.members.length,
            0,
        );
        const memberHighlights = highlighted.slice(memberIndex, memberIndex + memberCount);
        definitionIndex += item.declarations.length;
        memberIndex += memberCount;

        return renderLibraryItem(item, declarationHighlights, memberHighlights, order);
    });

    // reject route collisions before writing any generated pages
    const routes = new Set(items.map((item) => item.route));
    if (routes.size !== items.length) {
        throw new Error("duplicate canonical library item route");
    }

    // require every emitted module link to name one generated document
    const documentRoutes = new Set([catalog, ...modules].map((document) => document.route));
    for (const moduleItem of moduleItems.values()) {
        for (const item of moduleItem) {
            const targets = item.kind === "namespace" ? documentRoutes : routes;
            if (item.route != undefined && !targets.has(item.route)) {
                throw new Error(`missing generated library route: ${item.route}`);
            }
        }
    }

    return { documents: [catalog, ...modules], items };
}

/// Index exported names that resolve to one item.
function libraryReferences(items) {
    const routes = new Map();
    const ambiguous = new Set();

    // collect one destination for each unambiguous export name
    for (const item of items) {
        const route = routes.get(item.name);
        if (route == undefined) {
            routes.set(item.name, item.route);
        } else if (route !== item.route) {
            ambiguous.add(item.name);
        }
    }

    // remove names that could link to more than one declaration
    for (const name of ambiguous) {
        routes.delete(name);
    }

    return Object.fromEntries(routes);
}

/// Build the public items exposed from one module.
function libraryModuleItems(module, namespaceRoutes) {
    const items = new Map();

    // group overload declarations by exported name and declaration kind
    for (const exported of module.exports) {
        for (const declaration of exported.declarations) {
            const key = `${declaration.kind}\0${exported.name}`;
            let item = items.get(key);
            if (item == undefined) {
                item = {
                    declarations: [],
                    kind: declaration.kind,
                    module,
                    name: exported.name,
                    route: "",
                };
                items.set(key, item);
            }
            item.declarations.push(declaration);
        }

        // link namespace exports to a documented public module when one exists
        if (exported.namespace != undefined) {
            const route = namespaceRoutes.get(exported.namespace);
            items.set(`namespace\0${exported.name}`, {
                declarations: [],
                kind: "namespace",
                module,
                name: exported.name,
                route,
            });
        }
    }

    for (const item of items.values()) {
        item.identity = libraryItemIdentity(item);
    }

    return [...items.values()];
}

/// Select the most specific module for each declaration.
function canonicalLibraryItems(modules, moduleItems) {
    const canonical = new Map();

    for (const module of modules) {
        const items = moduleItems.get(module.specifier);
        if (items == undefined) {
            throw new Error(`missing indexed items for ${module.specifier}`);
        }

        for (const item of items) {
            if (item.kind === "namespace") {
                continue;
            }
            const current = canonical.get(item.identity);
            if (current == undefined || libraryItemScore(item) > libraryItemScore(current)) {
                canonical.set(item.identity, item);
            }
        }
    }

    return canonical;
}

/// Return an item identity from its declaration locations.
function libraryItemIdentity(item) {
    if (item.kind === "namespace") {
        return `${item.module.specifier}\0namespace\0${item.name}`;
    }
    const declarations = item.declarations
        .map((declaration) => {
            const source = declaration.source;

            return `${source.path}:${source.line}:${source.column}:${source.endLine}:${source.endColumn}`;
        })
        .sort()
        .join("|");

    return `${item.kind}\0${item.name}\0${declarations}`;
}

/// Prefer the public module closest to an item's authored source.
function libraryItemScore(item) {
    const sourcePath = item.declarations[0]?.source.path;
    const modulePath = item.module.path;
    if (sourcePath == undefined || modulePath == undefined) {
        return 0;
    }
    if (sourcePath === modulePath) {
        return 10_000;
    }

    const sourceSegments = sourcePath.split("/");
    const moduleSegments = dirname(modulePath).split("/");
    let shared = 0;
    while (shared < sourceSegments.length && sourceSegments[shared] === moduleSegments[shared]) {
        shared += 1;
    }

    return shared * 100 + moduleSegments.length;
}

/// Render the standard library catalog.
function renderLibraryCatalog(reference, index) {
    const moduleRows = reference.modules.map((module) => {
        const moduleRoute = libraryModuleRoute(module.specifier);

        return `<li><a href="${moduleRoute}"><code>${escapeHtml(module.specifier)}</code><span>${module.exports.length} exports</span></a></li>`;
    }).join("");
    const markdown = `${index.markdown}

${reference.modules.map((module) =>
        `- [${module.specifier}](${libraryModuleRoute(module.specifier)}) — ${module.exports.length} exports`
    ).join("\n")}`;
    const html = `${index.html}<div class="reference-catalog"><ul class="reference-module-list">${moduleRows}</ul></div>`;
    const searchSections = [
        ...index.searchSections,
        {
            depth: 1,
            id: "modules",
            text: reference.modules.map((module) => module.specifier).join(" "),
            title: "Modules",
        },
    ];

    return {
        ...index,
        html,
        markdown,
        searchSections,
        searchText: searchSections.map((section) => section.text).join(" "),
        tableOfContents: index.tableOfContents,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Render a standard library module index.
function renderLibraryModule(module, items, order) {
    const route = libraryModuleRoute(module.specifier);
    const groups = libraryItemGroups(items);
    const sections = groups.map((group) => {
        const rows = group.items.map((item) => {
            const name = `<code>${escapeHtml(item.name)}</code>`;
            const identity = item.route == undefined
                ? name
                : `<a href="${escapeAttribute(item.route)}">${name}</a>`;
            const description = libraryItemDescription(item);

            return `<li><div>${identity}<span>${escapeHtml(description)}</span></div></li>`;
        }).join("");

        return `<section class="reference-group"><h2 id="${group.id}">${group.title}</h2><ol class="reference-item-list">${rows}</ol></section>`;
    }).join("");
    const markdown = renderLibraryModuleMarkdown(module, groups);
    const description = `${module.specifier} standard library module.`;
    const searchSections = [{
        depth: 1,
        id: module.specifier,
        text: `${description} ${items.map((item) => `${item.name} ${libraryItemDescription(item)}`).join(" ")}`,
        title: module.specifier,
    }];
    const html = `<div class="reference-module"><p class="reference-import">import <code>${escapeHtml(module.specifier)}</code></p>${sections}</div>`;
    const path = libraryModulePath(module.specifier);

    return {
        assets: [],
        description,
        file: libraryReferenceFile,
        headings: searchSections,
        html,
        markdown,
        markdownRoute: `/docs/${path}`,
        order,
        path,
        route,
        searchSections,
        searchText: searchSections.map((section) => section.text).join(" "),
        tableOfContents: groups.map((group) => ({
            depth: 2,
            id: group.id,
            text: group.title,
        })),
        textRoute: `/docs/${path.replace(/\.md$/, ".txt")}`,
        title: module.specifier,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Group a module's exports by declaration kind.
function libraryItemGroups(items) {
    const groups = [];

    for (const kind of libraryKindOrder) {
        const grouped = items
            .filter((item) => item.kind === kind)
            .sort((left, right) => left.name.localeCompare(right.name));
        if (grouped.length === 0) {
            continue;
        }
        const title = libraryKindTitles.get(kind);
        if (title == undefined) {
            throw new Error(`missing library item group title: ${kind}`);
        }
        groups.push({ id: libraryKindSlug(kind), items: grouped, title });
    }

    const groupedCount = groups.reduce((count, group) => count + group.items.length, 0);
    if (groupedCount !== items.length) {
        const unknown = items.find((item) => !libraryKindTitles.has(item.kind));
        throw new Error(`unsupported library declaration kind: ${unknown?.kind}`);
    }

    return groups;
}

/// Render a standard library item.
function renderLibraryItem(item, declarationHighlights, memberHighlights, order) {
    const documentation = [...new Set(item.declarations
        .map((declaration) => declaration.documentation)
        .filter((value) => value != undefined))]
        .map((value) => renderLibraryDocumentation(value, item.route))
        .join("");
    const declarations = item.declarations.map((declaration, index) =>
        renderLibraryDeclaration(declaration, declarationHighlights[index])
    ).join("");
    const members = item.declarations.flatMap((declaration) => declaration.members);
    const memberHeading = item.kind === "enum" ? "Variants" : "Members";
    const memberId = memberHeading.toLowerCase();
    const memberDocumentation = renderLibraryMembers(
        members,
        memberHighlights,
        item.route,
        memberHeading,
        memberId,
    );
    const importStatement = `import { ${item.name} } from "${item.module.specifier}";`;
    const html = `<div class="reference-item"><p class="reference-import"><code>${escapeHtml(importStatement)}</code></p>${documentation}${declarations}${memberDocumentation}</div>`;
    const markdown = renderLibraryItemMarkdown(item);
    const description = libraryItemDescription(item);
    const path = libraryItemPath(item.module.specifier, item.kind, item.name);
    const tableOfContents = members.length === 0
        ? []
        : [{ depth: 2, id: memberId, text: memberHeading }];

    return {
        assets: [],
        description,
        file: libraryReferenceFile,
        headings: [{ depth: 1, id: libraryNameSlug(item.name), text: item.name }],
        html,
        lead: undefined,
        markdown,
        markdownRoute: `/docs/${path}`,
        module: item.module,
        moduleRoute: libraryModuleRoute(item.module.specifier),
        moduleTitle: libraryModuleTitle(item.module.specifier),
        order,
        path,
        route: item.route,
        searchContext: item.module.specifier,
        searchKind: "symbol",
        searchSections: [{
            depth: 1,
            id: libraryNameSlug(item.name),
            text: `${item.name} ${description} ${item.declarations.map((declaration) => declaration.signature.text).join(" ")} ${members.map((member) => `${member.signature.text} ${member.documentation ?? ""}`).join(" ")}`,
            title: item.name,
        }],
        searchText: `${item.name} ${description}`,
        tableOfContents,
        textRoute: `/docs/${path.replace(/\.md$/, ".txt")}`,
        title: item.name,
        tokens: tokenEstimateFor(plainTextFor(markdown)),
    };
}

/// Render one item declaration and its exact source location.
function renderLibraryDeclaration(declaration, highlighted) {
    if (highlighted == undefined) {
        throw new Error(`missing declaration highlight: ${declaration.signature.text}`);
    }

    return `<div class="reference-declaration">${renderReferenceCode(highlighted, declaration.source)}</div>`;
}

/// Render public member signatures and their authored documentation.
function renderLibraryMembers(members, highlighted, route, heading, id) {
    if (members.length === 0) {
        return "";
    }
    const rows = members.map((member, index) => {
        const signature = highlighted[index];
        if (signature == undefined) {
            throw new Error(`missing member highlight: ${member.signature.text}`);
        }
        const id = `member-${index + 1}`;
        const documentation = member.documentation == undefined
            ? ""
            : renderLibraryDocumentation(member.documentation, route);

        return `<div id="${escapeAttribute(id)}"><dt><code>${signature}</code></dt><dd>${documentation}</dd></div>`;
    }).join("");

    return `<section class="reference-member-section"><h2 id="${id}">${heading}</h2><dl class="reference-members">${rows}</dl></section>`;
}

/// Render documentation attached to one library item.
function renderLibraryDocumentation(documentation, route) {
    if (documentation == undefined) {
        return "";
    }

    return renderMarkdown(documentation, {
        assets: [],
        documentDirectory,
        kind: "document",
        markdownDirectory: dirname(libraryReferenceFile),
        ownHeadings: new Set(),
        route,
        slug: route,
        sourceRoutes: new Map(),
    });
}

/// Render one highlighted public definition.
function renderReferenceCode(highlighted, source) {
    const lines = highlighted.split("\n").map((line, index) => {
        const text = line === "" ? " " : line;

        return `<span class="markdown-code-line" data-publication-line><span class="markdown-code-gutter" data-publication-gutter>${index + 1}</span><span class="markdown-code-text" data-publication-code>${text}</span></span>`;
    }).join("");

    const location = librarySourceLocation(source);
    const sourceLink = source.path == undefined
        ? ""
        : `<a href="${escapeAttribute(librarySourceRoute(source))}" rel="external noopener noreferrer" target="_blank">${escapeHtml(location)}</a>`;

    return `<figure class="markdown-code reference-signature" data-publication-listing><figcaption data-publication-caption><span class="markdown-code__title reference-source" data-publication-caption-title>${sourceLink}</span></figcaption><pre data-publication-body tabindex="0"><code class="markdown-code-lines" data-publication-lines>${lines}</code></pre></figure>`;
}

/// Render one declaration and its public members as a definition.
function libraryDefinition(declaration) {
    if (declaration.members.length === 0) {
        return librarySignature(declaration);
    }

    const punctuation = declaration.kind === "enum" ? "," : ";";
    let text = `${declaration.signature.text} {\n`;
    const tokens = librarySignature(declaration).tokens;
    let length = Buffer.byteLength(text);

    // append member signatures while translating their semantic intervals
    for (const member of declaration.members) {
        const indent = "    ";
        text += indent;
        length += Buffer.byteLength(indent);

        for (const token of librarySignature(member).tokens) {
            tokens.push({
                ...token,
                start: length + token.start,
                end: length + token.end,
            });
        }

        const line = `${member.signature.text}${punctuation}\n`;
        text += line;
        length += Buffer.byteLength(line);
    }

    text += "}";

    return { text, tokens };
}

/// Classify the declaration name in a signature.
function librarySignature(declaration) {
    const name = declaration.signature.name;
    const tokens = name == undefined
        ? []
        : [{ ...name, kind: librarySemanticKind(declaration.kind) }];

    return { text: declaration.signature.text, tokens };
}

/// Return the semantic highlighter category for one declaration kind.
function librarySemanticKind(kind) {
    if (["class", "enum", "interface", "newtype_interface", "struct"].includes(kind)) {
        return kind;
    }

    if (["function", "method", "constructor"].includes(kind)) {
        return kind === "function" ? "function" : "method";
    }

    if (["associated_const", "enum_member", "field", "property"].includes(kind)) {
        return kind === "enum_member" ? "enum_member" : "property";
    }

    if (["module", "namespace"].includes(kind)) {
        return "namespace";
    }

    if (["constant", "variable"].includes(kind)) {
        return "variable";
    }

    return "type";
}

/// Render one portable Markdown module index.
function renderLibraryModuleMarkdown(module, groups) {
    const sections = groups.map((group) => {
        const rows = group.items.map((item) => {
            const description = libraryItemDescription(item);
            const name = item.route == undefined ? item.name : `[${item.name}](${item.route})`;

            return `- ${name} — ${description}`;
        }).join("\n");

        return `## ${group.title}\n\n${rows}`;
    }).join("\n\n");

    return `# ${module.specifier}\n\n\`import ${module.specifier}\`\n\n${sections}\n`;
}

/// Render one portable Markdown item reference.
function renderLibraryItemMarkdown(item) {
    const documentation = [...new Set(item.declarations
        .map((declaration) => declaration.documentation)
        .filter((value) => value != undefined))]
        .join("\n\n");
    const declarations = item.declarations.map((declaration) => {
        const definition = libraryDefinition(declaration);
        const location = librarySourceLocation(declaration.source);

        return `\`\`\`ds title="${item.name}"\n${definition.text}\n\`\`\`\n\n[${location}](${librarySourceRoute(declaration.source)})`;
    }).join("\n\n");
    const members = item.declarations
        .flatMap((declaration) => declaration.members)
        .map((member) => `### \`${member.signature.text}\`\n\n${member.documentation ?? ""}`)
        .join("\n\n");
    const memberHeading = item.kind === "enum" ? "Variants" : "Members";
    const memberSection = members === "" ? "" : `\n\n## ${memberHeading}\n\n${members}`;

    return `# ${item.name}\n\n\`import { ${item.name} } from "${item.module.specifier}";\`\n\n${documentation}\n\n${declarations}${memberSection}\n`;
}

/// Return the canonical site route for one public module specifier.
function libraryModuleRoute(specifier) {
    const suffix = specifier === "destack" ? "destack" : specifier.slice("destack:".length);

    return `${moduleCatalogRoute}${suffix}/`;
}

/// Return the concise public title for one standard library module.
function libraryModuleTitle(specifier) {
    return specifier === "destack" ? specifier : specifier.slice("destack:".length);
}

/// Return the generated portable source path for one public module.
function libraryModulePath(specifier) {
    const route = libraryModuleRoute(specifier).slice("/docs/".length);

    return `${route}index.md`;
}

/// Return the canonical site route for one public library item.
function libraryItemRoute(specifier, kind, name) {
    return `${libraryModuleRoute(specifier)}${libraryKindSlug(kind)}/${libraryNameSlug(name)}/`;
}

/// Return the generated portable source path for one public library item.
function libraryItemPath(specifier, kind, name) {
    const route = libraryItemRoute(specifier, kind, name).slice("/docs/".length);

    return `${route}index.md`;
}

/// Return the URL segment for one declaration kind.
function libraryKindSlug(kind) {
    return kind.replaceAll("_", "-");
}

/// Return the URL segment for one exported identifier.
function libraryNameSlug(name) {
    const slug = name
        .replaceAll(/([a-z0-9])([A-Z])/g, "$1-$2")
        .replaceAll("_", "-")
        .toLowerCase();
    if (!/^[a-z0-9$]+(?:-[a-z0-9$]+)*$/.test(slug)) {
        throw new Error(`invalid library item name: ${name}`);
    }

    return slug;
}

/// Return the first authored sentence for one public item.
function libraryItemDescription(item) {
    const documentation = item.declarations
        .map((declaration) => declaration.documentation)
        .find((value) => value != undefined);
    if (documentation == undefined) {
        return `${item.name} ${item.kind.replaceAll("_", " ")}.`;
    }
    const text = plainTextFor(documentation).replaceAll(/\s+/g, " ").trim();
    const sentence = text.match(/^.*?[.!?](?:\s|$)/)?.[0]?.trim();

    return sentence ?? text;
}

/// Return the display path for one public module.
function libraryModuleDisplay(packageName, path) {
    if (path == undefined) {
        return undefined;
    }
    const suffix = path.startsWith("src/") ? path.slice("src/".length) : path;

    return `${packageName}://${suffix}`;
}

/// Return the repository source URL for one declaration.
function librarySourceRoute(source) {
    const path = `language/library/${source.path}`;
    const end = source.endLine === source.line ? "" : `-L${source.endLine}`;

    return `https://github.com/destack-sh/destack/blob/main/${path}#L${source.line}${end}`;
}

/// Return the repository-relative source path and inclusive line range.
function librarySourceLocation(source) {
    if (source.path == undefined) {
        return "";
    }
    const path = `language/library/${source.path}`;
    const end = source.endLine === source.line ? "" : `:${source.endLine}`;

    return `${path}:${source.line}${end}`;
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

/// Read and validate every blog source.
