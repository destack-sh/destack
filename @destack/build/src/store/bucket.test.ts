import { MemoryBucket } from "@destack/bucket/test";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { Digest } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { BuildBucket } from "./bucket.ts";
import { PackageStore } from "./store.ts";

test("write a build once, read it back by its manifest's digest, and refuse a build the bucket lacks", async () => {
    // write a build, then again a minute later into the same bucket
    await using simulation = testSimulation();
    const { crypto, clock } = simulation.environment("test");
    const build = await MemoryBuild.write(
        new Map([["src/index.js", new TextEncoder().encode("export {};\n")]]),
        {
            package: Package.parse({
                id: "package-01996ab0-0000-7000-8000-000000000001",
                name: "@example/app",
                version: "2026.10.0",
            }),
        },
        crypto,
    );
    const digest = await PackageStore.digest(build.manifest, crypto);
    const bucket = new MemoryBucket(clock);
    const builds = new BuildBucket(bucket, crypto);
    await builds.write(build, digest);
    const first = await bucket.list({ prefix: "build/" });
    simulation.clock.set(clock.now() + 60_000);
    await builds.write(build, digest);
    const second = await bucket.list({ prefix: "build/" });

    // read the same manifest and verified files back, and refuse another digest
    const read = await builds.read(digest);
    const other = Digest.parse("1".repeat(64));
    expect({
        uploaded: second.files.map((file) => file.uploaded),
        manifest: read.manifest,
        source: new TextDecoder().decode(await read.load("src/index.js")),
        missing: await refusal(builds.read(other)),
    }).toEqual({
        uploaded: first.files.map((file) => file.uploaded),
        manifest: build.manifest,
        source: "export {};\n",
        missing: [
            "INTERNAL_SERVER_ERROR",
            "BUILD_NOT_IN_BUCKET",
            `the bucket has no build ${other}`,
        ],
    });
});
