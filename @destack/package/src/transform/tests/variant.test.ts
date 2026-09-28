import { expect, test } from "@destack/test";
import { greeting, loaded } from "./fixture/greeting.ts";
import { importVariants, requireBase, resolveVariant } from "../variant.ts";

/** Report the paths a test filesystem holds. */
function files(...paths: string[]) {
    return (path: string) => paths.includes(path);
}

test("load a module's server variant with its base exports under the server target", () => {
    expect([greeting, loaded]).toEqual(["hello", "server"]);
});

test("resolve relative imports to the target's variant only where one exists", () => {
    const exists = files("/package/src/page.server.ts", "/package/src/page.browser.ts");

    expect([
        resolveVariant("./page.ts", "/package/src/index.ts", "server", exists),
        resolveVariant("./page.ts", "/package/src/index.ts", "browser", exists),
        resolveVariant("./note.ts", "/package/src/index.ts", "server", exists),
        resolveVariant("./page.ts", "/package/src/page.server.ts", "server", exists),
        resolveVariant("@destack/db", "/package/src/index.ts", "server", exists),
    ]).toEqual([
        "/package/src/page.server.ts",
        "/package/src/page.browser.ts",
        undefined,
        undefined,
        undefined,
    ]);
    expect(() =>
        resolveVariant("./page.server.ts", "/package/src/view.ts", "browser", exists),
    ).toThrow(
        "browser module /package/src/view.ts imports server variant /package/src/page.server.ts",
    );
});

test("name the target's variants in a module's relative imports and re-exports", () => {
    const exists = files("/package/src/page.server.ts");
    const code = [
        'import { page } from "./page.ts";',
        'export * from "./page.ts";',
        'import { note } from "./note.ts";',
        'import { sql } from "@destack/db";',
    ].join("\n");

    // rewrite the imports of modules with a variant, and leave modules without one
    expect([
        importVariants(code, "/package/src/index.ts", "server", exists),
        importVariants(code, "/package/src/index.ts", "browser", exists),
        importVariants(
            'export * from "./page.ts";',
            "/package/src/page.server.ts",
            "server",
            exists,
        ),
    ]).toEqual([
        [
            'import { page } from "./page.server.ts";',
            'export * from "./page.server.ts";',
            'import { note } from "./note.ts";',
            'import { sql } from "@destack/db";',
        ].join("\n"),
        undefined,
        undefined,
    ]);
});

test("require a variant to re-export its base", () => {
    // accept a variant re-exporting its base and any module that is no variant
    requireBase('export * from "./page.ts";\nexport const handler = 1;', "/src/page.server.ts");
    requireBase("export const page = 1;", "/src/page.ts");

    // reject a variant replacing its base without its exports
    expect(() => requireBase("export const handler = 1;", "/src/page.server.ts")).toThrow(
        'variant /src/page.server.ts must re-export its base with export * from "./page.ts"',
    );
});
