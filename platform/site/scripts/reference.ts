import { escapeAttribute, escapeHtml } from "../src/content/html.ts";
import type { DocumentationPage } from "./page.ts";
import type { MarkdownContext } from "./markdown.ts";

import { renderContentList } from "../src/content/presentation.ts";
import { renderListing } from "../src/content/listing.ts";
import { marked } from "marked";
import { appendDocumentSections } from "./sections.ts";
import { existsSync, readFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { highlightCode, highlightCodeFragments } from "./highlight.ts";
import { headingsFor, parseFrontmatter, renderMarkdown, searchSectionsFor } from "./markdown.ts";
import { plainTextFor, tokenEstimateFor } from "./text.ts";

/// A source interval in a compiler package reference.
type SourceLocation = {
    path?: string | null;
    line: number;
    column: number;
    endLine: number;
    endColumn: number;
};

/// A documented declaration, including overloads and public members.
type Declaration = {
    name?: string | null;
    kind: string;
    documentation?: string | null;
    members: Declaration[];
    signature: {
        text: string;
        name?: { start: number; end: number } | null;
    };
    source: SourceLocation;
};

/// Authored package or module Markdown.
type Readme = {
    path: string;
    markdown: string;
};

/// A public module or a namespace reached through its exports.
type PackageModule = {
    module: string;
    specifier: string;
    path?: string | null;
    exports: {
        name: string;
        declarations: Declaration[];
        namespace?: string | null;
    }[];
    parentRoute?: string;
    importSpecifier?: string;
    importName?: string;
};

/// The compiler's checked package documentation artifact.
export type PackageReference = {
    schemaVersion?: number;
    package: { name: string };
    modules: PackageModule[];
    namespaces?: (Omit<PackageModule, "specifier">)[];
};

/// An exported name grouped with its overload declarations.
type PackageItem = {
    name: string;
    kind: string;
    module: PackageModule;
    declarations: Declaration[];
    route: string;
    description?: string;
};

/// The package's published landing page and source locations.
type PackageIndex = {
    route: string;
    path: string;
    title: string;
    order: number;
};
/// Filesystem and repository locations used to render a package.
type PackageOptions = {
    directory: string;
    referenceFile: string;
    sourceUrl: string;
};

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const libraryReferenceFile = join(
    repositoryDirectory,
    "platform/site/.generated/library-reference.json",
);
const moduleCatalogPath = "language/standard-library";
/// The standard library module catalogue.
export const moduleCatalogRoute = `/docs/${moduleCatalogPath}/`;
/// Read the standard library package reference.
export function readLibraryReference() {
    if (!existsSync(libraryReferenceFile)) {
        throw new Error("missing generated library reference, run `just platform/site/generate`");
    }

    const reference: PackageReference = JSON.parse(readFileSync(libraryReferenceFile, "utf8"));
    if (
        reference.schemaVersion !== 8 || !Array.isArray(reference.modules) ||
        !Array.isArray(reference.namespaces)
    ) {
        throw new Error(`unsupported library reference in ${libraryReferenceFile}`);
    }

    return reference;
}

/// Render a package catalog, module indexes, and items.
export function renderPackageDocuments(
    reference: PackageReference,
    index: PackageIndex,
    options: PackageOptions,
): { documents: DocumentationPage[]; items: (DocumentationPage & { module: PackageModule })[] } {
    const documentDirectory = options.directory;
    const packageReferenceFile = options.referenceFile;
    const moduleCatalogRoute = index.route;
    const order = index.order;
    const namespacePages = new Map<string, string>();
    const packageModules = [...reference.modules];
    const publicModuleRoutes = new Map(
        reference.modules
            .filter((module) => module.specifier !== reference.package.name)
            .map((module) => [module.specifier, packageModuleRoute(module.specifier)]),
    );
    const namespaceRoutes = new Map<string, string>(reference.modules.map((module) => [
        module.module,
        packageModuleRoute(module.specifier),
    ]));
    const namespaceReferences = new Map(
        (reference.namespaces ?? []).map((namespace) => [namespace.module, namespace]),
    );

    // publish each reachable namespace once; aliases and cycles link to its existing page
    for (let cursor = 0; cursor < packageModules.length; cursor += 1) {
        const parent = packageModules[cursor];
        for (const exported of parent.exports) {
            if (exported.namespace == undefined || namespaceRoutes.has(exported.namespace)) {
                continue;
            }
            const namespace = namespaceReferences.get(exported.namespace);
            if (namespace == undefined) {
                throw new Error(`missing exported namespace: ${exported.namespace}`);
            }
            const specifier = `${parent.specifier}.${exported.name}`;
            const parentRoute = packageModuleRoute(parent.specifier);
            const route = `${parentRoute}namespace/${packageNameSlug(exported.name)}/`;
            namespacePages.set(specifier, route);
            namespaceRoutes.set(exported.namespace, route);
            packageModules.push({
                ...namespace,
                specifier,
                parentRoute,
                importSpecifier: parent.importSpecifier ?? parent.specifier,
                importName: parent.importName ?? exported.name,
            });
        }
    }

    const moduleItems = new Map(packageModules.map((module) => [
        module.specifier,
        packageModuleItems(module, namespaceRoutes),
    ]));
    const canonicalItems = canonicalPackageItems(packageModules, moduleItems);

    // point every re-export at one canonical item page
    for (const items of moduleItems.values()) {
        for (const item of items) {
            if (item.kind === "namespace") {
                continue;
            }
            const canonical = canonicalItems.get(packageItemIdentity(item));
            if (canonical == undefined) {
                throw new Error(`missing canonical package item: ${packageItemIdentity(item)}`);
            }
            item.route = packageItemRoute(canonical.module.specifier, item.kind, item.name);
        }
    }

    // describe namespace inventories with their actual exported-name count
    for (const items of moduleItems.values()) {
        for (const item of items.filter((item) => item.kind === "namespace")) {
            const target = packageModules.find((module) =>
                packageModuleRoute(module.specifier) === item.route
            );
            if (target == undefined) throw new Error(`missing namespace document: ${item.route}`);
            item.description = exportCount(target.exports.length);
        }
    }

    // authored Markdown belongs to site generation, not the compiler reference
    const packageReadme = readReadme("README.md");
    const moduleReadmes = new Map(packageModules.map((module) => [
        module.specifier,
        module.path != undefined && basename(module.path) === "index.ds"
            ? readReadme(join(dirname(module.path), "README.md"))
            : undefined,
    ]));
    const readmes: [Readme, string][] = [];
    const introductions: [Readme | undefined, string][] = [
        [packageReadme, index.route],
        ...packageModules.filter((module) => module.specifier !== reference.package.name)
            .map((
                module,
            ): [Readme | undefined, string] => [
                moduleReadmes.get(module.specifier),
                packageModuleRoute(module.specifier),
            ]),
    ];
    for (const [readme, route] of introductions) {
        if (readme != undefined) readmes.push([readme, route]);
    }
    const sourceRoutes = new Map(readmes.map(([readme, route]) => [
        resolve(documentDirectory, readme.path),
        {
            route,
            headings: new Set(
                headingsFor(parseReadme(readme.markdown, readme.path).markdown).map((heading) =>
                    heading.id
                ),
            ),
        },
    ]));
    // highlight inventory names together with compiler-derived declaration kinds
    const inventoryItems = [...moduleItems.values()].flat();
    const inventoryNames = highlightCodeFragments(
        inventoryItems.map((item) => item.name),
        "ds",
        inventoryItems.map((item) => [
            { start: 0, end: Buffer.byteLength(item.name), kind: packageSemanticKind(item.kind) },
        ]),
    );
    const highlightedNames = new Map(
        inventoryItems.map((item, index) => [item, inventoryNames[index]]),
    );

    const introduction = renderReadme(packageReadme, index.route);
    const catalog = {
        ...renderPackageCatalog(reference, { ...index, ...introduction }),
        searchKind: "catalog",
        file: packageReferenceFile,
        markdownRoute: `/docs/${index.path}`,
        textRoute: `/docs/${index.path.replace(/\.md$/, ".txt")}`,
    };
    const modules = packageModules.filter((module) => module.specifier !== reference.package.name)
        .map((module) => {
            const items = moduleItems.get(module.specifier);
            if (items == undefined) {
                throw new Error(`missing indexed items for ${module.specifier}`);
            }

            return renderPackageModule(module, items, order);
        });

    // highlight definitions and members as independent snippets
    const canonical = [...canonicalItems.values()]
        .sort((left, right) => left.route.localeCompare(right.route));
    const references = packageReferences(canonical);
    const definitions = canonical.flatMap((item) =>
        item.declarations.map((declaration) => packageDefinition(declaration))
    );
    const memberSignatures = canonical.flatMap((item) =>
        item.declarations.flatMap((declaration) =>
            declaration.members.map((member) => packageSignature(member))
        )
    );
    const highlighted = highlightCodeFragments(
        [...definitions, ...memberSignatures].map((definition) => definition.text),
        "ds",
        [...definitions, ...memberSignatures].map((definition) => definition.tokens),
        references,
    );

    // highlight complete import statements together as one valid source file
    const imports = canonical.length === 0 ? [] : highlightCode(
        canonical.map((item) => packageImport(item.module, item.name)).join("\n"),
        "ds",
    ).split("\n");

    // assign each highlighted definition and member back to its canonical item
    let definitionIndex = 0;
    let memberIndex = definitions.length;
    const items = canonical.map((item, index) => {
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

        return renderPackageItem(
            item,
            declarationHighlights,
            memberHighlights,
            imports[index],
            order,
        );
    });

    // reject route collisions before writing any generated pages
    const routes = new Set(items.map((item) => item.route));
    if (routes.size !== items.length) {
        throw new Error("duplicate canonical package item route");
    }

    // require every emitted module link to name one generated document
    const documentRoutes = new Set([catalog, ...modules].map((document) => document.route));
    for (const moduleItem of moduleItems.values()) {
        for (const item of moduleItem) {
            const targets = item.kind === "namespace" ? documentRoutes : routes;
            if (item.route != undefined && !targets.has(item.route)) {
                throw new Error(`missing generated package route: ${item.route}`);
            }
        }
    }

    // keep the package landing page a flat catalog of public modules
    return { documents: [catalog, ...modules], items };

    /// Read optional authored Markdown from the package's content directory.
    function readReadme(path: string): Readme | undefined {
        const file = resolve(documentDirectory, path);
        if (!existsSync(file)) return undefined;

        return { path, markdown: readFileSync(file, "utf8") };
    }

    /// Render authored Markdown and resolve package-relative links and assets.
    function renderReadme(readme: Readme | undefined, route: string) {
        const { markdown, metadata } = parseReadme(readme?.markdown ?? "", readme?.path ?? route);
        const headings = headingsFor(markdown);
        const context: MarkdownContext = {
            assets: [],
            documentDirectory,
            kind: "document",
            markdownDirectory: dirname(resolve(documentDirectory, readme?.path ?? "README.md")),
            ownHeadings: new Set(headings.map((heading) => heading.id)),
            route,
            slug: route,
            sourceRoutes,
        };
        const html = renderMarkdown(markdown, context);

        return {
            assets: context.assets,
            description: metadata.description,
            html,
            markdown,
            headings,
            searchSections: searchSectionsFor(markdown),
            tableOfContents: headings.filter((heading) => heading.depth > 1),
        };
    }

    /// Render the package catalog.
    function renderPackageCatalog(
        reference: PackageReference,
        index: PackageIndex & ReturnType<typeof renderReadme>,
    ) {
        const publicModules = reference.modules.filter((module) =>
            module.specifier !== reference.package.name
        );
        const moduleRows = renderContentList(publicModules.map((module) => ({
            title: module.specifier,
            href: packageModuleRoute(module.specifier),
            meta: exportCount(module.exports.length),
            code: true,
        })));
        const rootItems = moduleItems.get(reference.package.name) ?? [];

        return appendDocumentSections(index, [
            ...rootItems.length > 0 ? packageExportSections(rootItems) : [],
            ...publicModules.length > 0
                ? [{
                    title: "Modules",
                    html: `<div class="reference-catalog">${moduleRows}</div>`,
                    markdown: publicModules.map((module) =>
                        `- [${module.specifier}](${packageModuleRoute(module.specifier)}) — ${
                            exportCount(module.exports.length)
                        }`
                    ).join("\n"),
                }]
                : [],
        ]);
    }

    /// Render the same export inventory on package and module pages.
    function packageExportSections(items: PackageItem[]) {
        const sorted = [...items].sort((left, right) => left.name.localeCompare(right.name));
        const rows = sorted.map((item) => {
            const name = `<code class="reference-name">${highlightedNames.get(item)}</code>`;
            const description = item.description ?? packageItemDescription(item);
            const contents = `${name}${
                description
                    ? `<span class="reference-summary">${escapeHtml(description)}</span>`
                    : ""
            }`;

            return `<li><a href="${escapeAttribute(item.route)}">${contents}</a></li>`;
        }).join("");

        return [{
            title: "Exports",
            html: rows === ""
                ? ""
                : `<div class="reference-group"><ol class="reference-item-list">${rows}</ol></div>`,
            markdown: sorted.map((item) => {
                const description = item.description ?? packageItemDescription(item);

                return `- [${item.name}](${item.route})${
                    description === "" ? "" : ` — ${description}`
                }`;
            }).join("\n"),
        }];
    }

    /// Render a package module index.
    function renderPackageModule(module: PackageModule, items: PackageItem[], order: number) {
        const route = packageModuleRoute(module.specifier);
        const readme = renderReadme(moduleReadmes.get(module.specifier), route);
        if (module.importName != undefined) {
            const statement = packageImport(module, module.importName);
            readme.html =
                renderListing(highlightCode(statement, "ds"), { label: "Destack import" }) +
                readme.html;
            readme.markdown =
                `# ${module.specifier}\n\n\`\`\`ds\n${statement}\n\`\`\`\n\n${readme.markdown}`;
        }
        const content = appendDocumentSections({
            ...readme,
            markdown: readme.markdown || `# ${module.specifier}`,
        }, packageExportSections(items));
        // nest beneath the nearest published module in the import path
        let parentSpecifier = module.specifier;
        let parentRoute = moduleCatalogRoute;
        while (parentSpecifier.includes("/")) {
            parentSpecifier = parentSpecifier.slice(0, parentSpecifier.lastIndexOf("/"));
            const publishedRoute = publicModuleRoutes.get(parentSpecifier);
            if (publishedRoute != undefined) {
                parentRoute = publishedRoute;
                break;
            }
        }
        const path = packageModulePath(module.specifier);

        return {
            ...content,
            parentRoute: module.parentRoute ?? parentRoute,
            file: packageReferenceFile,
            markdownRoute: `/docs/${path}`,
            order,
            path,
            route,
            textRoute: `/docs/${path.replace(/\.md$/, ".txt")}`,
            title: module.specifier,
        };
    }

    /// Render a package item.
    function renderPackageItem(
        item: PackageItem,
        declarationHighlights: string[],
        memberHighlights: string[],
        importHighlight: string,
        order: number,
    ) {
        const documentation = [
            ...new Set(
                item.declarations
                    .map((declaration) => declaration.documentation)
                    .filter((value) => value != undefined),
            ),
        ]
            .map((value) => renderPackageDocumentation(value, item.route))
            .join("");
        const declarations = item.declarations.map((declaration, index) =>
            renderPackageDeclaration(declaration, declarationHighlights[index])
        ).join("");
        const members = item.declarations.flatMap((declaration) => declaration.members);
        const memberHeading = item.kind === "enum" ? "Variants" : "Members";
        const memberId = memberHeading.toLowerCase();
        const memberDocumentation = renderPackageMembers(
            members,
            memberHighlights,
            item.route,
            memberHeading,
            memberId,
        );
        const html = `<div class="reference-item">${
            renderReferenceCode(importHighlight)
        }${documentation}${declarations}${memberDocumentation}</div>`;
        const markdown = renderPackageItemMarkdown(item);
        const description = packageItemDescription(item);
        const path = packageItemPath(item.module.specifier, item.kind, item.name);
        const tableOfContents = members.length === 0
            ? []
            : [{ depth: 2, id: memberId, text: memberHeading }];

        return {
            assets: [],
            description,
            file: packageReferenceFile,
            headings: [{ depth: 1, id: packageNameSlug(item.name), text: item.name }],
            html,
            lead: undefined,
            markdown,
            markdownRoute: `/docs/${path}`,
            module: item.module,
            moduleRoute: packageModuleRoute(item.module.specifier),
            order,
            path,
            route: item.route,
            searchContext: item.module.specifier,
            searchKind: "symbol",
            searchSections: [{
                depth: 1,
                id: packageNameSlug(item.name),
                text: `${item.name} ${description} ${
                    item.declarations.map((declaration) => declaration.signature.text).join(" ")
                } ${
                    members.map((member) =>
                        `${member.signature.text} ${member.documentation ?? ""}`
                    ).join(" ")
                }`,
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
    function renderPackageDeclaration(declaration: Declaration, highlighted: string) {
        if (highlighted == undefined) {
            throw new Error(`missing declaration highlight: ${declaration.signature.text}`);
        }

        return `<div class="reference-declaration">${
            renderReferenceCode(highlighted, declaration.source)
        }</div>`;
    }

    /// Render public member signatures and their authored documentation.
    function renderPackageMembers(
        members: Declaration[],
        highlighted: string[],
        route: string,
        heading: string,
        id: string,
    ) {
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
                : renderPackageDocumentation(member.documentation, route);

            return `<div id="${
                escapeAttribute(id)
            }"><dt><code>${signature}</code></dt><dd>${documentation}</dd></div>`;
        }).join("");

        return `<section class="reference-member-section"><h2 id="${id}">${heading}</h2><dl class="reference-members">${rows}</dl></section>`;
    }

    /// Render documentation attached to one package item.
    function renderPackageDocumentation(documentation: string | null | undefined, route: string) {
        if (documentation == undefined) {
            return "";
        }

        return renderMarkdown(documentation, {
            assets: [],
            documentDirectory,
            kind: "document",
            markdownDirectory: dirname(packageReferenceFile),
            ownHeadings: new Set(),
            route,
            slug: route,
            sourceRoutes: new Map(),
        });
    }

    /// Render one highlighted public definition.
    function renderReferenceCode(highlighted: string, source?: SourceLocation) {
        const detail = source?.path == undefined
            ? undefined
            : `L${source.line}${source.endLine === source.line ? "" : `–${source.endLine}`}`;

        return renderListing(highlighted, {
            title: source?.path ?? undefined,
            href: source?.path == undefined ? undefined : packageSourceRoute(source),
            detail,
            label: "Destack code",
        });
    }

    /// Render one portable Markdown item reference.
    function renderPackageItemMarkdown(item: PackageItem) {
        const documentation = [
            ...new Set(
                item.declarations
                    .map((declaration) => declaration.documentation)
                    .filter((value) => value != undefined),
            ),
        ]
            .join("\n\n");
        const declarations = item.declarations.map((declaration) => {
            const definition = packageDefinition(declaration);
            const location = packageSourceLocation(declaration.source);

            return `\`\`\`ds title="${item.name}"\n${definition.text}\n\`\`\`\n\n[${location}](${
                packageSourceRoute(declaration.source)
            })`;
        }).join("\n\n");
        const members = item.declarations
            .flatMap((declaration) => declaration.members)
            .map((member) => `### \`${member.signature.text}\`\n\n${member.documentation ?? ""}`)
            .join("\n\n");
        const memberHeading = item.kind === "enum" ? "Variants" : "Members";
        const memberSection = members === "" ? "" : `\n\n## ${memberHeading}\n\n${members}`;

        return `# ${item.name}\n\n\`\`\`ds\n${
            packageImport(item.module, item.name)
        }\n\`\`\`\n\n${documentation}\n\n${declarations}${memberSection}\n`;
    }

    /// Return the canonical site route for one public module specifier.
    function packageModuleRoute(specifier: string) {
        const namespaceRoute = namespacePages.get(specifier);
        if (namespaceRoute != undefined) return namespaceRoute;
        const suffix = packageModuleTitle(specifier);

        return specifier === reference.package.name
            ? moduleCatalogRoute
            : `${moduleCatalogRoute}${suffix}/`;
    }

    /// Return the concise public title for one package module.
    function packageModuleTitle(specifier: string) {
        return specifier === reference.package.name
            ? specifier
            : specifier.slice(reference.package.name.length + 1);
    }

    /// Return the generated portable source path for one public module.
    function packageModulePath(specifier: string) {
        const route = packageModuleRoute(specifier).slice("/docs/".length);

        return `${route}index.md`;
    }

    /// Return the canonical site route for one public package item.
    function packageItemRoute(specifier: string, kind: string, name: string) {
        return `${packageModuleRoute(specifier)}${packageKindSlug(kind)}/${packageNameSlug(name)}/`;
    }

    /// Return the generated portable source path for one public package item.
    function packageItemPath(specifier: string, kind: string, name: string) {
        const route = packageItemRoute(specifier, kind, name).slice("/docs/".length);

        return `${route}index.md`;
    }

    /// Return the repository source URL for one declaration.
    function packageSourceRoute(source: SourceLocation) {
        const path = source.path;
        const end = source.endLine === source.line ? "" : `-L${source.endLine}`;

        return `${options.sourceUrl}${path}#L${source.line}${end}`;
    }
}

/// Read optional frontmatter without requiring metadata in package READMEs.
export function parseReadme(source: string, file: string) {
    const parsed = source.startsWith("---\n")
        ? parseFrontmatter(source, file)
        : { markdown: source, metadata: {} };
    const title = headingsFor(parsed.markdown).find((heading) => heading.depth === 1)?.text ?? "";
    const description =
        marked.lexer(parsed.markdown).find((token): token is import("marked").Tokens.Paragraph =>
            token.type === "paragraph"
        )?.text ?? "";

    return {
        markdown: parsed.markdown,
        metadata: {
            title,
            description: plainTextFor(description).replaceAll(/\s+/g, " ").trim(),
            ...parsed.metadata,
        },
    };
}

/// Index exported names that resolve to one item.
function packageReferences(items: PackageItem[]) {
    const routes = new Map<string, string>();
    const ambiguous = new Set<string>();

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
function packageModuleItems(module: PackageModule, namespaceRoutes: Map<string, string>) {
    const items = new Map<string, PackageItem>();

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
            if (route == undefined) {
                throw new Error(`missing namespace route: ${exported.namespace}`);
            }
            items.set(`namespace\0${exported.name}`, {
                declarations: [],
                kind: "namespace",
                module,
                name: exported.name,
                route,
            });
        }
    }

    return [...items.values()];
}

/// Select the most specific module for each declaration.
function canonicalPackageItems(modules: PackageModule[], moduleItems: Map<string, PackageItem[]>) {
    const canonical = new Map<string, PackageItem>();

    for (const module of modules) {
        const items = moduleItems.get(module.specifier);
        if (items == undefined) {
            throw new Error(`missing indexed items for ${module.specifier}`);
        }

        for (const item of items) {
            if (item.kind === "namespace") {
                continue;
            }
            const current = canonical.get(packageItemIdentity(item));
            if (
                current == undefined ||
                (current.module.importName != undefined && item.module.importName == undefined) ||
                ((current.module.importName == undefined) ===
                        (item.module.importName == undefined) &&
                    packageItemScore(item) > packageItemScore(current))
            ) {
                canonical.set(packageItemIdentity(item), item);
            }
        }
    }

    return canonical;
}

/// Return an item identity from its declaration locations.
function packageItemIdentity(item: PackageItem) {
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
function packageItemScore(item: PackageItem) {
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

/// Render one declaration and its public members as a definition.
export function packageDefinition(declaration: Declaration) {
    // retain authored documentation and adjust semantic byte offsets after comments
    const comment = documentationComment(declaration.documentation);
    const signature = packageSignature(declaration);
    const offset = Buffer.byteLength(comment);
    const tokens = signature.tokens.map((token) => ({
        ...token,
        start: token.start + offset,
        end: token.end + offset,
    }));
    let text = comment + signature.text;
    if (declaration.members.length === 0) {
        return { text, tokens };
    }

    // document each public member without exposing implementation bodies
    const punctuation = declaration.kind === "enum" ? "," : ";";
    text += " {\n";
    for (const [index, member] of declaration.members.entries()) {
        if (index > 0) text += "\n";
        text += documentationComment(member.documentation, "    ") + "    ";
        const length = Buffer.byteLength(text);
        for (const token of packageSignature(member).tokens) {
            tokens.push({ ...token, start: length + token.start, end: length + token.end });
        }
        text += `${member.signature.text}${punctuation}\n`;
    }
    text += "}";

    return { text, tokens };
}

/// Reconstitute documentation comments from the compiler's public API model.
function documentationComment(documentation: string | null | undefined, indent = ""): string {
    if (!documentation) return "";

    return documentation.split("\n").map((line) => `${indent}///${line ? ` ${line}` : ""}\n`).join(
        "",
    );
}

/// Classify the declaration name in a signature.
function packageSignature(declaration: Declaration) {
    const name = declaration.signature.name;
    const tokens = name == undefined
        ? []
        : [{ ...name, kind: packageSemanticKind(declaration.kind) }];

    return { text: declaration.signature.text, tokens };
}

/// Return the semantic highlighter category for one declaration kind.
function packageSemanticKind(kind: string) {
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

/// Return the URL segment for one declaration kind.
function packageKindSlug(kind: string) {
    return kind.replaceAll("_", "-");
}

/// Return the URL segment for one exported identifier.
function packageNameSlug(name: string) {
    const slug = name
        .replaceAll(/([a-z0-9])([A-Z])/g, "$1-$2")
        .replaceAll("_", "-")
        .toLowerCase();
    if (!/^[a-z0-9$]+(?:-[a-z0-9$]+)*$/.test(slug)) {
        throw new Error(`invalid package item name: ${name}`);
    }

    return slug;
}

/// Return the first authored sentence for one public item.
function packageItemDescription(item: PackageItem) {
    const documentation = item.declarations
        .map((declaration) => declaration.documentation)
        .find((value) => value != undefined);
    if (documentation == undefined) {
        return "";
    }
    const text = plainTextFor(documentation).replaceAll(/\s+/g, " ").trim();
    const sentence = text.match(/^.*?[.!?](?:\s|$)/)?.[0]?.trim();

    return sentence ?? text;
}

/// Return the repository-relative source path and inclusive line range.
function packageSourceLocation(source: SourceLocation) {
    if (source.path == undefined) {
        return "";
    }
    const path = source.path;
    const end = source.endLine === source.line ? "" : `:${source.endLine}`;

    return `${path}:${source.line}${end}`;
}

/// Describe an inventory without assuming plural exports.
function exportCount(count: number) {
    return `${count} ${count === 1 ? "export" : "exports"}`;
}

/// Import an item directly or through its publicly exported namespace.
function packageImport(module: PackageModule, name: string) {
    return `import { ${module.importName ?? name} } from "${
        module.importSpecifier ?? module.specifier
    }";`;
}
