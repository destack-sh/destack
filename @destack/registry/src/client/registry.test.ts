import { PackageId, PackageVersion } from "@destack/package";
import { BuildReader, BuildWriter, Lock } from "@destack/package/manifest";
import { TestBuildStore } from "@destack/build/test";
import { testSimulation } from "@destack/object/test";
import { Digest, found, schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Registry } from "./registry.ts";
import registryPackage from "../package.ts";

/** The forge's URL, which serves the builds below the registry's mount. */
const FORGE = "https://forge.test";

/** The package the traced build builds. */
const ANSWER = {
    id: PackageId.parse("package-00000000-0000-7000-8000-00000000a001"),
    name: "@example/answer",
    version: "2026.9.0",
};

/** The finance package at a version. */
function finance(version: string) {
    return PackageVersion.parse({
        id: "package-01a10720-c4d6-7559-816b-99f193c28c3c",
        name: "@destack/finance",
        version,
    });
}

test("answer a shipped release by its version or prerelease, and ask the forges for any other release or tag", async () => {
    // ship a prerelease beside forges that refuse every call
    const environment = testSimulation().environment("registry");
    const prerelease = "2026.9.0-dev.20260912100000.0123456789ab";
    const registry = new Registry(
        (owner) => Promise.reject(new TypeError(`asked the forge of ${JSON.stringify(owner)}`)),
        environment,
        [{ package: finance(prerelease), lock: Digest.parse("1".repeat(64)) }],
    );
    const { id } = finance("2026.9.0");
    const asked = `asked the forge of ${JSON.stringify({ package: id })}`;

    // find the shipped one without a forge
    expect({
        shipped: await registry.find(id, "2026.9.0"),
        exact: await registry.find(id, prerelease),
        other: await registry.find(id, "2026.10.0").catch((error: unknown) => String(error)),
        tag: await registry.find(id, "latest").catch((error: unknown) => String(error)),
    }).toEqual({
        shipped: "1".repeat(64),
        exact: "1".repeat(64),
        other: `TypeError: ${asked}`,
        tag: `TypeError: ${asked}`,
    });
});

test("trace a push to a forge, a pull from it, and the first read of its build alone", async () => {
    // write a build in memory and store it under a lock
    await using simulation = testSimulation();
    const environment = simulation.environment("registry");
    await using source = await TestBuildStore.create(environment);
    await using forge = await TestBuildStore.create(environment);
    await using pulled = await TestBuildStore.create(environment);
    const bytes = new Map<string, Uint8Array<ArrayBuffer>>();
    const writer = new BuildWriter(
        { write: async (path, contents) => void bytes.set(path, contents) },
        environment.crypto,
    );
    await writer.write("src/index.ts", new TextEncoder().encode("export const answer = 42;\n"));
    const manifest = await writer.finish({
        destack: "2026.11.0",
        role: "app",
        package: ANSWER,
        outputs: {},
    });
    const build = new BuildReader(
        manifest,
        async (path) => new Blob([found(bytes, path)]).stream(),
        environment.crypto,
    );
    const digest = await source.put(build);
    const lock = await source.putLock(Lock.of(digest));

    // serve the forge's builds, granting every push and pull
    const mount = new URL(`${FORGE}${registryPackage.mounts.builds.path}/`);
    const access = { pull: async () => {}, push: async () => {} };
    const registry = new Registry(
        async () => ({
            url: FORGE,
            space: "space-00000000-0000-7000-8000-000000000001",
            fetch: async (request: Request) => forge.fetch(request, mount, access),
        }),
        environment,
    );

    // push the lock, pull it into another store, and read its build twice
    const account = schema
        .identifier("account")
        .parse("account-00000000-0000-7000-8000-000000000001");
    await registry.push(source, lock, account);
    await registry.pull(pulled, ANSWER.id, lock, "all");
    await registry.read(ANSWER.id, digest);
    await registry.read(ANSWER.id, digest);

    // read the registry's spans in the order they ended, and its durations
    const spans = simulation.telemetry.spans
        .filter((span) => span.instrumentationScope.name === registryPackage.name)
        .map((span) => span.name);
    const counts = Object.fromEntries(
        Object.entries(await simulation.telemetry.counts()).filter(
            ([key]) => key.startsWith("destack.registry.") && key.endsWith(".count"),
        ),
    );

    // expect one span per transfer, the second read answered by the opened reader
    expect({ spans, counts }).toEqual({
        spans: ["destack.registry.push", "destack.registry.pull", "destack.registry.read"],
        counts: {
            "destack.registry.push.duration.count": 1,
            "destack.registry.pull.duration.count": 1,
            "destack.registry.read.duration.count": 1,
        },
    });
});
