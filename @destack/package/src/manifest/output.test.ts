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
