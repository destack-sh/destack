import { expect, test } from "@destack/test";
import { found, schema } from "@destack/schema";
import { PackageFile } from "../file/file.ts";
import { PackageError } from "../error/error.ts";
import { Package } from "../definition/package.ts";
import { BuildReader } from "./reader.ts";
import type { PackageManifest } from "./manifest.ts";

/** The declaring package's first release. */
const owner = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000001",
    name: "@destack/setting",
    version: "2026.9.0",
});

/** The declaring package's next release, held beside the first. */
const upgraded = Package.parse({ ...owner, version: "2026.9.1" });

/** A package declaring nothing. */
const other = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000002",
    name: "@destack/audit",
    version: "2026.9.0",
});

/** A package whose constructor also describes the declaring package's kind. */
const notification = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000003",
    name: "@destack/notification",
    version: "2026.9.0",
});

/** An empty inventory file of a build. */
const EMPTY = await PackageFile.describe(
    "manifest/empty.json",
    "application/json",
    new Uint8Array(),
);

/** Record a declaration of a package's constructor as a build records it. */
function declaration(
    declaring: Package,
    constructor: Package,
    kind: string,
    name: string,
    description: object,
) {
    return {
        name,
        kind,
        package: declaring,
        constructor: { package: constructor, symbol: { module: "src/setting.ts", name: "define" } },
        symbol: { package: other, symbol: { module: "src/index.ts", name } },
        source: { file: "src/index.ts", line: 0, column: 0 },
        description,
    };
}

test("read the descriptions of one kind across every version of a package's declarations", async () => {
    // hold one collection per version of the declaring package, the first mixing in other kinds
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const descriptions: PackageManifest["descriptions"] = {};
    for (const [domain, constructor, declarations] of [
        [
            "setting",
            owner,
            [
                declaration(owner, owner, "setting", "language", { name: "language" }),
                declaration(owner, owner, "schedule", "nightly", { cron: "0 0 * * *" }),
                declaration(owner, notification, "setting", "notification.mention", {
                    name: "notification.mention",
                }),
            ],
        ],
        [
            "setting-upgraded",
            upgraded,
            [declaration(upgraded, upgraded, "setting", "theme", { name: "theme" })],
        ],
    ] as const) {
        const bytes = new TextEncoder().encode(JSON.stringify(declarations));
        const file = await PackageFile.describe(
            `manifest/${domain}.json`,
            "application/json",
            bytes,
        );
        files.set(file.path, bytes);
        descriptions[domain] = { package: constructor, file };
    }
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: owner,
        language: "typescript",
        dependencies: EMPTY,
        descriptions,
        outputs: {},
        files: EMPTY,
        sourceMaps: EMPTY,
    };
    const reader = new BuildReader(manifest, async (path) => found(files, path));

    // read both versions' settings with the notification's, and nothing for another package
    const item = schema.object({ name: schema.string() }).strict();
    const read = async (id: typeof owner.id) =>
        (await reader.declared(id, "setting", item)).map((declared) => declared.description);
    expect([await read(owner.id), await read(other.id)]).toEqual([
        [{ name: "language" }, { name: "notification.mention" }, { name: "theme" }],
        [],
    ]);
});

test("read a build's own declarations across its domains, and reference its upgrade", async () => {
    // hold the build's declaration beside a dependency's in two domains, and its upgrade
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const store = async (path: string, value: unknown) => {
        const bytes = new TextEncoder().encode(JSON.stringify(value));
        files.set(path, bytes);

        return {
            package: owner,
            file: await PackageFile.describe(path, "application/json", bytes),
        };
    };
    const own = declaration(owner, owner, "setting", "language", { name: "language" });
    const dependency = {
        ...declaration(owner, owner, "setting", "theme", { name: "theme" }),
        symbol: { package: owner, symbol: { module: "src/index.ts", name: "theme" } },
    };
    const inventory = async (name: string) =>
        (await store(`manifest/${name}.json`, name === "dependencies" ? {} : [])).file;
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: other,
        language: "typescript",
        outputs: {},
        dependencies: await inventory("dependencies"),
        files: await inventory("files"),
        sourceMaps: await inventory("sourceMaps"),
        descriptions: {
            setting: await store("manifest/setting.json", [own]),
            theme: await store("manifest/theme.json", [dependency]),
        },
        upgrade: await store("manifest/upgrade.json", { from: "2026.8.0", steps: [] }),
    };
    const reader = new BuildReader(manifest, async (path) => found(files, path));

    // keep the build's own declaration, and reference the upgrade after the collections
    expect([await reader.declarations(), reader.references().map((file) => file.path)]).toEqual([
        [own],
        [
            "manifest/dependencies.json",
            "manifest/files.json",
            "manifest/sourceMaps.json",
            "manifest/setting.json",
            "manifest/theme.json",
            "manifest/upgrade.json",
        ],
    ]);
});

test("open a remote build by its manifest digest, and refuse other bytes and unsafe addresses", async () => {
    // serve a build's manifest and one file over a fake endpoint
    const content = new TextEncoder().encode("export {};");
    const manifest: PackageManifest = {
        formatVersion: 1,
        package: owner,
        language: "typescript",
        dependencies: EMPTY,
        descriptions: {},
        outputs: {},
        files: EMPTY,
        sourceMaps: EMPTY,
    };
    const bytes = new TextEncoder().encode(JSON.stringify(manifest));
    const served = new Map([
        ["https://registry.example/build/manifest.json", bytes],
        ["https://registry.example/build/files/src/index.js", content],
    ]);
    const fetch = async (input: URL) => {
        const body = served.get(input.href);

        return body === undefined ? new Response(null, { status: 404 }) : new Response(body);
    };
    const digest = (await PackageFile.describe("manifest.json", "application/json", bytes)).digest;

    // read the manifest and a file below the base, which gains its trailing slash
    const reader = await BuildReader.open(
        { manifest: digest, url: "https://registry.example/build" },
        fetch,
    );
    expect(reader.manifest).toEqual(manifest);
    expect(await reader.load("src/index.js")).toEqual(content);

    // refuse a manifest of other bytes, an address with credentials and a failed read
    const forged = (await PackageFile.describe("manifest.json", "application/json", content))
        .digest;
    await expect(
        BuildReader.open({ manifest: forged, url: "https://registry.example/build/" }, fetch),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "file digest mismatch: manifest.json"));
    await expect(
        BuildReader.open(
            { manifest: digest, url: "https://user:secret@registry.example/build/" },
            fetch,
        ),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "invalid package URL"));
    await expect(
        BuildReader.open({ manifest: digest, url: "https://registry.example/missing/" }, fetch),
    ).rejects.toThrow(new PackageError("INVALID_FILE", "package read failed: HTTP 404"));
});
