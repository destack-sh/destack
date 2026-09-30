import { expect, test } from "@destack/test";
import { Package } from "@destack/package";
import type { PackageOutput } from "@destack/package/manifest";
import type { TestDeclaration } from "@destack/test/inspect";
import { encodeDescription, serializeDescriptions } from "./manifest.ts";

/** The package declaring the thing kind. */
const owner = Package.parse({
    id: "package-01996ab0-0000-7000-8000-00000000000a",
    name: "@example/owner",
    version: "2026.9.0",
});

/** The package declaring the greeting kind and its constructor. */
const other = Package.parse({
    ...owner,
    id: "package-01996ab0-0000-7000-8000-00000000000b",
    name: "@example/other",
});

/** The package declaring the greeting. */
const app = Package.parse({
    ...owner,
    id: "package-01996ab0-0000-7000-8000-00000000000c",
    name: "@example/app",
});

/** Describe the app's greeting as one kind of a package, made by the other package's constructor. */
function describe(kind: string, declaring: Package) {
    return {
        name: "greeting",
        kind,
        package: declaring,
        constructor: { package: other, symbol: { module: "index.ts", name: "defineGreeting" } },
        symbol: { package: app, symbol: { module: "src/index.ts", name: "greeting" } },
        source: { file: "src/index.ts", line: 2, column: 24 },
        description: { name: "greeting" },
    };
}

test("keep each description in the collection of the package declaring its kind", async () => {
    // serialize the greeting as the other package's kind and the owner's kind for one output
    const greeting = describe("greeting", other);
    const thing = describe("thing", owner);
    const output = { descriptions: {} } as PackageOutput;
    const manifest = await serializeDescriptions(
        {
            modules: [],
            declarations: [greeting, thing],
            tests: [],
            testPackage: app,
            modulePackage: app,
            selections: new Map([["server", { modules: [], declarations: [0, 1], tests: [] }]]),
        },
        { server: output },
    );

    // expect one collection per declaring package, selected by the output
    expect([
        Object.fromEntries(
            Object.entries(manifest.descriptions).map(([name, collection]) => [
                name,
                collection.package,
            ]),
        ),
        Object.fromEntries(manifest.files),
        output.descriptions,
    ]).toEqual([
        { other, owner },
        {
            "manifest/other.json": encodeDescription([greeting]),
            "manifest/owner.json": encodeDescription([thing]),
        },
        { other: [0], owner: [0] },
    ]);
});

test("keep test declarations apart from the declarations of the same package", async () => {
    // serialize the app's tests beside a declaration of the app's own kind
    const thing = describe("thing", app);
    const tests = [
        {
            kind: "test",
            name: "greets",
            file: "src/index.test.ts",
            start: 0,
            end: 9,
            modifiers: [],
        },
    ] satisfies TestDeclaration[];
    const output = { descriptions: {} } as PackageOutput;
    const manifest = await serializeDescriptions(
        {
            modules: [],
            declarations: [thing],
            tests,
            testPackage: app,
            modulePackage: app,
            selections: new Map([["server", { modules: [], declarations: [0], tests: [0] }]]),
        },
        { server: output },
    );

    // expect the declaration collection and the test file, each selected by the output
    expect([
        Object.keys(manifest.descriptions),
        manifest.tests?.file.path,
        Object.fromEntries(manifest.files),
        [output.descriptions, output.tests],
    ]).toEqual([
        ["app"],
        "manifest/tests.json",
        {
            "manifest/app.json": encodeDescription([thing]),
            "manifest/tests.json": encodeDescription(tests),
        },
        [{ app: [0] }, [0]],
    ]);
});
