import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { describeFile } from "../file/file.ts";
import { Package } from "../definition/package.ts";
import { PackageReader } from "./reader.ts";
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

/** Record a declaration of a package's constructor as a build records it. */
function declaration(constructor: Package, kind: string, name: string, description: object) {
    return {
        name,
        kind,
        constructor: { package: constructor, symbol: { module: "src/setting.ts", name: "define" } },
        symbol: { package: other, symbol: { module: "src/index.ts", name } },
        source: { file: "src/index.ts", line: 0, column: 0 },
        description,
    };
}

test("read the descriptions of one kind across every version of a package's declarations", async () => {
    // hold one collection per version of the declaring package, the first mixing in another kind
    const files = new Map<string, Uint8Array<ArrayBuffer>>();
    const descriptions: PackageManifest["descriptions"] = {};
    for (const [domain, constructor, declarations] of [
        [
            "setting",
            owner,
            [
                declaration(owner, "setting", "language", { name: "language" }),
                declaration(owner, "schedule", "nightly", { cron: "0 0 * * *" }),
            ],
        ],
        [
            "setting-upgraded",
            upgraded,
            [declaration(upgraded, "setting", "theme", { name: "theme" })],
        ],
    ] as const) {
        const bytes = new TextEncoder().encode(JSON.stringify(declarations));
        const file = await describeFile(`manifest/${domain}.json`, "application/json", bytes);
        files.set(file.path, bytes);
        descriptions[domain] = { package: constructor, file };
    }
    const reader = new PackageReader({ descriptions } as PackageManifest, async (path) =>
        files.get(path)!,
    );

    // read both versions' settings, skip the schedule, and read nothing for another package
    const item = schema.object({ name: schema.string() }).strict();
    expect([
        await reader.declared(owner.id, "setting", item),
        await reader.declared(other.id, "setting", item),
    ]).toEqual([[{ name: "language" }, { name: "theme" }], []]);
});
