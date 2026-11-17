import { expect, test } from "@destack/test";
import { ImportMap } from "./output.ts";

/** A map of a build's imports at the top, and a dependency's own imports in its scope. */
const MAP: ImportMap = {
    imports: { "@example/view": "/view-1/index.js", "@example/kit/": "/kit/src/" },
    scopes: { "/view-1/": { "@example/view": "/view-2/index.js" } },
};

test("rewrite the mapped specifiers of static, re-exporting and dynamic imports, leaving the others", async () => {
    // rewrite a module of the build and one of the dependency's scope
    const code = [
        'import { view } from "@example/view";',
        'export * from "@example/kit/button.js";',
        'const later = import("@example/view");',
        'import { env } from "cloudflare:workers";',
        'import "./local.js";',
        "const pattern = import(`@example/kit/${name}.js`);",
        'const text = "import x from \\"@example/view\\"";',
    ].join("\n");

    // map the top-level and prefixed specifiers, the dependency's own in its scope
    expect([
        await ImportMap.apply(MAP, code, "/app/index.js"),
        await ImportMap.apply(MAP, 'import "@example/view";', "/view-1/chunk.js"),
    ]).toEqual([
        [
            'import { view } from "/view-1/index.js";',
            'export * from "/kit/src/button.js";',
            'const later = import("/view-1/index.js");',
            'import { env } from "cloudflare:workers";',
            'import "./local.js";',
            "const pattern = import(`@example/kit/${name}.js`);",
            'const text = "import x from \\"@example/view\\"";',
        ].join("\n"),
        'import "/view-2/index.js";',
    ]);
});

test("list the modules a module imports or names by URL, and whether it imports one by a computed name", async () => {
    // import a mapped package, a mapped subpath, a platform module, relative modules and a computed name
    const code = [
        'import { view } from "@example/view";',
        'export * from "@example/kit/button.js";',
        'import { env } from "cloudflare:workers";',
        'import "./local.js";',
        'const later = import("../shared/chunk.js");',
        "const here = import.meta.url;",
        'const worker = new Worker(new URL("./worker.js", import.meta.url));',
    ].join("\n");
    const computed = "const pattern = import(`./languages/${name}.js`);";

    // list each literal import where it resolves, and mark the module with a computed import open
    expect([
        await ImportMap.imports(MAP, code, "/app/src/index.js"),
        await ImportMap.imports(MAP, computed, "/app/src/index.js"),
    ]).toEqual([
        {
            modules: [
                "/view-1/index.js",
                "/kit/src/button.js",
                "/app/src/local.js",
                "/app/shared/chunk.js",
                "/app/src/worker.js",
            ],
            isOpen: false,
        },
        { modules: [], isOpen: true },
    ]);
});
