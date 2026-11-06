import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";
import { toJsonSchema } from "@destack/schema";
import { Definition, PackageDefinition } from "./definition.ts";

/** The published JSON Schema of destack.json, served per release. */
const SCHEMA_FILE = fileURLToPath(new URL("../../schemas/destack.json", import.meta.url));

test("publish the JSON Schema of destack.json as the package and workspace definitions declare it", async () => {
    // compare the checkout's file with the declaration, which vitest's update rewrites
    const described = `${JSON.stringify(toJsonSchema(Definition), null, 4)}\n`;
    await expect(described).toMatchFileSnapshot(SCHEMA_FILE);
});

test("read the packages whose build extensions compile a package, refusing an empty list", () => {
    // read a definition listing the style extension, then one listing none
    expect([
        PackageDefinition.read(definition(["@destack/style"])).extensions,
        (() => {
            try {
                return PackageDefinition.read(definition([]));
            } catch (error) {
                return error instanceof Error ? error.name : error;
            }
        })(),
    ]).toEqual([["@destack/style"], "ZodError"]);
});

test("read the modules a host starts, refusing an empty list and a path outside the package", () => {
    // read a definition listing a Worker script, then one listing none and one escaping the package
    const read = (entrypoints: readonly string[]) => {
        try {
            return PackageDefinition.read(
                JSON.stringify({ ...JSON.parse(definition(["@destack/style"])), entrypoints }),
            ).entrypoints;
        } catch (error) {
            return error instanceof Error ? error.name : error;
        }
    };

    expect([read(["src/worker.ts"]), read([]), read(["../outside.ts"])]).toEqual([
        ["src/worker.ts"],
        "ZodError",
        "ZodError",
    ]);
});

/** Write a destack.json listing extensions. */
function definition(extensions: readonly string[]): string {
    return JSON.stringify({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-0000000000a1",
        role: "library",
        language: "typescript",
        runtimes: ["browser"],
        extensions,
    });
}

test("refuse an app's server and providers, test kits and any app in a builtin, outside the importing package and its tests", () => {
    // import each subpath from a builtin, an app and a test, refusing as the layout says
    const app = { name: "@acme/notes", role: "app" } as const;
    const library = { name: "@acme/kit", role: "library" } as const;
    const importers = {
        builtin: { name: "@destack/machine", role: "builtin", isTest: false },
        app: { name: "@acme/board", role: "app", isTest: false },
        test: { name: "@acme/board", role: "app", isTest: true },
        own: { name: "@acme/notes", role: "app", isTest: false },
    } as const;
    const imports = [
        [app, "./client"],
        [app, "./server"],
        [app, "./provider/stripe"],
        [library, "./test"],
        [library, "."],
    ] as const;
    const outcomes = Object.fromEntries(
        Object.entries(importers).map(([key, importer]) => [
            key,
            imports.map(([imported, entry]) => {
                try {
                    PackageDefinition.requireImport(importer, imported, entry);

                    return "allowed";
                } catch (error) {
                    return error instanceof Error ? error.message : String(error);
                }
            }),
        ]),
    );

    // allow the importing package and tests everything, and an app only interfaces and libraries
    expect(outcomes).toEqual({
        builtin: [
            "builtin @destack/machine imports app @acme/notes/client",
            "@destack/machine imports @acme/notes/server, private to its app",
            "@destack/machine imports @acme/notes/provider/stripe, private to its app",
            "@destack/machine imports @acme/kit/test outside its tests",
            "allowed",
        ],
        app: [
            "allowed",
            "@acme/board imports @acme/notes/server, private to its app",
            "@acme/board imports @acme/notes/provider/stripe, private to its app",
            "@acme/board imports @acme/kit/test outside its tests",
            "allowed",
        ],
        test: ["allowed", "allowed", "allowed", "allowed", "allowed"],
        own: [
            "allowed",
            "allowed",
            "allowed",
            "@acme/notes imports @acme/kit/test outside its tests",
            "allowed",
        ],
    });
});

test("refuse an app without its package or with a free subpath, and a library with a package", () => {
    // check each role's exports, keeping each refusal's message
    const notes = PackageDefinition.parse({
        $schema: "https://destack.app/schemas/2026.9.0/destack.json",
        id: "package-01996ab0-0000-7000-8000-000000000a01",
        role: "app",
        language: "typescript",
    });
    const outcome = (role: PackageDefinition["role"], entries: readonly string[]) => {
        try {
            PackageDefinition.requireExports({ ...notes, role }, "@acme/notes", entries);

            return "allowed";
        } catch (error) {
            return error instanceof Error ? error.message : String(error);
        }
    };

    // allow an app's standard subpaths and providers, a builtin's free ones, and a library without a package
    expect({
        app: outcome("app", [
            "./package",
            "./object",
            "./server",
            "./provider/stripe",
            "./destack.json",
        ]),
        unpackaged: outcome("app", ["./object"]),
        root: outcome("app", ["./package", "."]),
        free: outcome("app", ["./package", "./oauth"]),
        library: outcome("library", [".", "./package"]),
        builtin: outcome("builtin", [".", "./package", "./cloudflare"]),
    }).toEqual({
        app: "allowed",
        unpackaged: "app @acme/notes exports no ./package",
        root: "app @acme/notes exports ., outside the standard subpaths",
        free: "app @acme/notes exports ./oauth, outside the standard subpaths",
        library: "library @acme/notes exports ./package",
        builtin: "allowed",
    });
});
