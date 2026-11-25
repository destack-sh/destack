import { join } from "node:path";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { testHost } from "@destack/test/host";
import { ConstructorCatalog, PackageReadings } from "./catalog.ts";

/** Write a package's manifest in a directory. */
async function write(directory: string, manifest: Record<string, string>): Promise<void> {
    const { files } = testHost();
    await files.mkdir(directory, { recursive: true });
    await files.writeTextFile(join(directory, "package.json"), JSON.stringify(manifest));
}

test("share a stored root's readings across builds and read the build's own anew", async () => {
    // write a build's package and a dependency placed in a tree's root
    const { files } = testHost();
    const root = await files.makeTemporaryDirectory("catalog-");
    onTestFinished(() => files.remove(root, { recursive: true }));
    const own = join(root, "own");
    const stored = join(root, "trees", "root-a", "node_modules", "@example", "dependency");
    await write(own, { name: "@example/own" });
    await write(stored, { name: "@example/dependency", version: "2026.11.0" });

    // read both in a build sharing the stored root's readings
    const readings = new PackageReadings();
    readings.retain(new Set([stored]));
    const read = async () => {
        const catalog = new ConstructorCatalog({}, files, readings);
        const located = [await catalog.locate(join(stored, "src")), await catalog.locate(own)];

        return located.map((location) => location.name);
    };
    const first = await read();

    // rename both packages, then read them in a second build
    await write(own, { name: "@example/renamed" });
    await write(stored, { name: "@example/changed", version: "2026.11.0" });
    const second = await read();

    // expect the stored root's reading kept, as stored roots never change, and the own package read anew
    expect([first, second, [...readings.locations.keys()]]).toEqual([
        ["@example/dependency", "@example/own"],
        ["@example/dependency", "@example/renamed"],
        [join(stored, "src")],
    ]);
});

test("forget the readings below roots a later tree no longer places", async () => {
    // read a package below one root, then take a tree with another
    const { files } = testHost();
    const root = await files.makeTemporaryDirectory("catalog-retain-");
    onTestFinished(() => files.remove(root, { recursive: true }));
    const first = join(root, "root-a", "node_modules", "@example", "dependency");
    const second = join(root, "root-b", "node_modules", "@example", "dependency");
    await write(first, { name: "@example/dependency", version: "2026.11.0" });
    await write(second, { name: "@example/dependency", version: "2026.11.0" });
    const readings = new PackageReadings();
    readings.retain(new Set([first, second]));
    await new ConstructorCatalog({}, files, readings).locate(first);
    await new ConstructorCatalog({}, files, readings).locate(second);
    readings.retain(new Set([second]));

    // expect only the reading below the root still placed
    expect([...readings.locations.keys()]).toEqual([second]);
});

test("read a package again after a failed reading, forgetting the failure", async () => {
    // locate a directory without a package, then write its package and locate it again
    const { files } = testHost();
    const root = await files.realPath(await files.makeTemporaryDirectory("catalog-retry-"));
    onTestFinished(() => files.remove(root, { recursive: true }));
    const catalog = new ConstructorCatalog({}, files, new PackageReadings());
    const missing = await refusal(catalog.locate(root));
    await write(root, { name: "@example/late" });

    // expect the failure, then the package
    expect([
        missing === "done" ? missing : missing.slice(0, 2),
        (await catalog.locate(root)).name,
    ]).toEqual([["BAD_REQUEST", "DEPENDENCY_INVALID"], "@example/late"]);
});
