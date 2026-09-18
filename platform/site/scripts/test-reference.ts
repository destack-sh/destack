import type { NavigationPage } from "./page.ts";
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { packageDefinition, parseReadme, renderPackageDocuments } from "./reference.ts";
import { appendDocumentSections } from "./sections.ts";
import { buildNavigation } from "./navigation.ts";

// exercise the same renderer with an ordinary package and nested public exports
const directory = mkdtempSync(join(tmpdir(), "destack-readme-"));
try {
    const readme = {
        path: "README.md",
        markdown: "# Relay\n\n[Binding](src/nested/README.md#binding)\n\n![Root](root.svg)\n",
    };
    const moduleReadme = { path: "src/README.md", markdown: "# API\n\n![API](api.svg)\n" };
    const bindingReadme = {
        path: "src/nested/README.md",
        markdown: "# Binding\n\n[Package](../../README.md#relay)\n",
    };
    for (const file of [readme, moduleReadme, bindingReadme]) {
        mkdirSync(join(directory, file.path, ".."), { recursive: true });
        writeFileSync(join(directory, file.path), file.markdown);
    }
    writeFileSync(join(directory, "root.svg"), "<svg/>");
    writeFileSync(join(directory, "src/api.svg"), "<svg/>");
    const reference = {
        package: { name: "relay" },
        modules: [
            { module: "relay://index.ds", specifier: "relay", path: "src/index.ds", exports: [] },
            {
                module: "relay://nested/index.ds",
                specifier: "relay/nested/binding",
                path: "src/nested/index.ds",
                exports: [],
            },
            {
                module: "relay://empty.ds",
                specifier: "relay/empty",
                path: "src/empty.ds",
                exports: [],
            },
        ],
    };
    const index = {
        route: "/docs/library/relay/",
        path: "library/relay/index.md",
        title: "Relay",
        order: 0,
    };
    const { documents, items } = renderPackageDocuments(reference, index, {
        directory,
        referenceFile: join(directory, "reference.json"),
        sourceUrl: "https://example.com/",
    });
    assert.deepEqual(documents.map((page) => page.route), [
        "/docs/library/relay/",
        "/docs/library/relay/nested/binding/",
        "/docs/library/relay/empty/",
    ]);
    assert.deepEqual(items, []);
    assert.equal(
        documents[1].html,
        '<h1 id="binding">Binding</h1><p><a href="/docs/library/relay/#relay">Package</a></p>\n<section class="document-section"><h2 id="exports">Exports</h2></section>',
    );
    assert.equal(documents[2].description, "");
    assert.deepEqual(documents[0].assets.map((asset) => [asset.path, asset.placeholder]), [
        [join(directory, "root.svg"), "__CONTENT_ASSET_0__"],
    ]);

    assert.deepEqual(documents[0].tableOfContents, [{ depth: 2, id: "modules", text: "Modules" }]);

    // paths without a published ancestor remain directly beneath the package
    documents.forEach((page, index) => {
        page.kind = index === 0 ? "chapter" : "module";
    });
    const roots = [
        { route: "/docs/", title: "Docs", kind: "chapter" },
        { route: "/docs/library/", title: "Libraries", kind: "chapter" },
    ];
    buildNavigation(
        [...roots, ...documents].sort((left, right) => left.route.localeCompare(right.route)),
        [],
    );
    assert.deepEqual(documents[1].navigation!.ancestors.map((page) => page.route), [
        "/docs/",
        "/docs/library/",
        "/docs/library/relay/",
    ]);
    // the catalog and each module expose the same sibling inventory
    const expectedRoutes = [
        "/docs/library/relay/",
        "/docs/library/relay/empty/",
        "/docs/library/relay/nested/binding/",
    ];
    for (const page of documents) {
        assert.deepEqual(page.navigation!.entries.map((entry) => entry.route), expectedRoutes);
    }
    assert.deepEqual(documents[1].tableOfContents, [{ depth: 2, id: "exports", text: "Exports" }]);
    assert.equal(documents.some((page) => page.html.includes('class="reference-import"')), false);

    // reference pages reuse their containing collection without inserting individual symbols
    documents[2].kind = "page";
    const symbol: NavigationPage = {
        route: `${documents[1].route}symbol/`,
        title: "Symbol",
        parentRoute: documents[1].route,
    };
    buildNavigation(
        [...roots, ...documents].sort((left, right) => left.route.localeCompare(right.route)),
        [symbol],
    );
    assert.deepEqual(symbol.navigation!.entries.map((entry) => entry.route), expectedRoutes);
    assert.deepEqual(documents[2].navigation!.entries.map((entry) => entry.route), expectedRoutes);

    // reject self-parenting before walking ancestors
    assert.throws(() =>
        buildNavigation([
            { route: "/docs/", title: "Docs", kind: "chapter" },
            { route: "/docs/self/", parentRoute: "/docs/self/", title: "Self" },
        ], []), { message: "invalid parent /docs/self/ for /docs/self/" });

    // published path ancestors determine hierarchy without introducing missing modules
    const nestedPages = renderPackageDocuments(
        {
            ...reference,
            modules: [...reference.modules, {
                module: "relay://nested.ds",
                specifier: "relay/nested",
                path: "src/nested.ds",
                exports: [],
            }, {
                module: "relay://deep.ds",
                specifier: "relay/nested/more/deep",
                path: "src/deep.ds",
                exports: [],
            }],
        },
        index,
        {
            directory,
            referenceFile: join(directory, "reference.json"),
            sourceUrl: "https://example.com/",
        },
    ).documents;
    nestedPages.forEach((page, index) => {
        page.kind = index === 0 ? "chapter" : "module";
    });
    buildNavigation(
        [...roots, ...nestedPages].sort((left, right) => left.route.localeCompare(right.route)),
        [],
    );
    assert.deepEqual(nestedPages[0].navigation!.entries.map(({ route, depth }) => [route, depth]), [
        ["/docs/library/relay/", 0],
        ["/docs/library/relay/empty/", 1],
        ["/docs/library/relay/nested/", 1],
        ["/docs/library/relay/nested/binding/", 2],
        ["/docs/library/relay/nested/more/deep/", 2],
    ]);
    assert.deepEqual(nestedPages[1].navigation!.ancestors.map((page) => page.route), [
        "/docs/",
        "/docs/library/",
        "/docs/library/relay/",
        "/docs/library/relay/nested/",
    ]);
    assert.deepEqual(
        nestedPages[0].markdown.split("\n").filter((line) => line.startsWith("- [")).map((line) =>
            line.split("]")[0]
        ),
        [
            "- [relay/nested/binding",
            "- [relay/empty",
            "- [relay/nested",
            "- [relay/nested/more/deep",
        ],
    );

    // generated headings share nesting and duplicate anchors with authored headings
    const composed = appendDocumentSections({
        markdown: "# API\n\n## Exports\n",
        html: '<h1 id="api">API</h1><h2 id="exports">Exports</h2>',
    }, [{
        title: "Exports",
        markdown: "",
        html: "",
        children: [{ title: "Writing", markdown: "Details.", html: "<p>Details.</p>" }],
    }]);
    assert.deepEqual(composed.tableOfContents, [
        { depth: 2, id: "exports", text: "Exports" },
        { depth: 2, id: "exports-2", text: "Exports" },
        { depth: 3, id: "writing", text: "Writing" },
    ]);
    assert.equal(
        composed.html,
        '<h1 id="api">API</h1><h2 id="exports">Exports</h2><section class="document-section"><h2 id="exports-2">Exports</h2><section class="document-section"><h3 id="writing">Writing</h3><p>Details.</p></section></section>',
    );
    assert.deepEqual(
        composed.searchSections.map(({ depth, id, title }) => ({ depth, id, text: title })),
        composed.headings,
    );

    assert.deepEqual(parseReadme("# Relay\n\nDescription.\n", "README.md"), {
        markdown: "# Relay\n\nDescription.\n",
        metadata: { title: "Relay", description: "Description." },
    });
    // namespace-only modules get pages, valid imports, and finite cyclic navigation
    const declaration = {
        name: "value",
        kind: "constant",
        documentation: "A value.",
        members: [],
        signature: { text: "export const value = 1", name: { start: 13, end: 18 } },
        source: { path: "src/tools.ds", line: 1, column: 1, endLine: 1, endColumn: 23 },
    };
    // a package with only a root module must expose its symbols on the landing page
    writeFileSync(join(directory, "README.md"), "# Relay\n");
    const rootOnly = renderPackageDocuments(
        {
            package: { name: "relay" },
            modules: [{
                module: "relay://index.ds",
                specifier: "relay",
                path: "src/index.ds",
                exports: [{ name: "value", declarations: [declaration] }],
            }],
        },
        index,
        {
            directory,
            referenceFile: join(directory, "reference.json"),
            sourceUrl: "https://example.com/",
        },
    );
    assert.deepEqual(rootOnly.documents.map((page) => page.route), [index.route]);
    assert.equal(
        rootOnly.documents[0].markdown,
        `# Relay\n\n## Exports\n\n- [value](${rootOnly.items[0].route}) — A value.\n`,
    );
    assert.deepEqual(rootOnly.documents[0].tableOfContents, [{
        depth: 2,
        id: "exports",
        text: "Exports",
    }]);

    // comments preserve Unicode text without shifting semantic identifier ranges
    const documented = packageDefinition({
        ...declaration,
        documentation: "A café value.\n\nMore detail.",
        members: [{ ...declaration, documentation: "Nested value." }],
    });
    assert.match(documented.text, /^\/\/\/ A café value\.\n\/\/\/\n\/\/\/ More detail\./);
    assert.match(documented.text, /    \/\/\/ Nested value\.\n    export const value/);
    for (const token of documented.tokens) {
        assert.equal(
            Buffer.from(documented.text).subarray(token.start, token.end).toString(),
            "value",
        );
    }
    assert.equal(
        packageDefinition({ ...declaration, documentation: undefined }).text,
        declaration.signature.text,
    );

    const namespaceReference = {
        package: { name: "relay" },
        modules: [{
            module: "relay://api.ds",
            specifier: "relay/api",
            path: "src/api.ds",
            exports: [
                { name: "tools", declarations: [], namespace: "relay://tools.ds" },
                { name: "alias", declarations: [], namespace: "relay://tools.ds" },
                { name: "value", declarations: [declaration] },
            ],
        }],
        namespaces: [{
            module: "relay://tools.ds",
            path: "src/tools.ds",
            exports: [
                { name: "parent", declarations: [], namespace: "relay://api.ds" },
                { name: "nested", declarations: [], namespace: "relay://nested.ds" },
                { name: "value", declarations: [declaration] },
            ],
        }, { module: "relay://nested.ds", path: "src/nested.ds", exports: [] }],
    };
    const namespaceResult = renderPackageDocuments(namespaceReference, index, {
        directory,
        referenceFile: join(directory, "reference.json"),
        sourceUrl: "https://example.com/",
    });
    const namespacePages = namespaceResult.documents;
    assert.equal(namespaceResult.items[0].module.specifier, "relay/api");
    assert.equal(namespacePages.length, 4);
    const tools = namespacePages.find((page) => page.title === "relay/api.tools");
    assert.ok(tools);
    assert.equal(tools.parentRoute, "/docs/library/relay/api/");
    assert.match(tools.markdown, /import \{ tools \} from "relay\/api";/);
    assert.match(namespacePages[1].html, /namespace\/tools\/">/);
    assert.match(namespacePages[1].html, /3 exports/);
    assert.ok(!namespacePages[0].html.includes("relay/api.tools"));
    assert.equal(namespacePages[3].parentRoute, tools.route);
    assert.match(tools.html, /href="\/docs\/library\/relay\/api\/"/);
} finally {
    rmSync(directory, { recursive: true, force: true });
}

console.log("Package README rendering passed.");
