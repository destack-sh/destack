import { expect, test } from "@destack/test";
import { PackageArchive } from "../src/pack/index.ts";
import { COMMIT, fixtureBuild } from "./fixture/package.ts";

test("pack a build into identical archives declaring its compiled exports and exact dependencies", async () => {
    // pack one build twice
    const build = await fixtureBuild("greeting");
    const first = await PackageArchive.pack(build, COMMIT);
    const second = await PackageArchive.pack(build, COMMIT);
    const list = await build.reader.distributed();
    const { bun, workerd } = build.manifest.outputs;
    if (bun === undefined || workerd === undefined) {
        throw new TypeError("greeting builds no bun or workerd output");
    }

    // declare the source types and the compiled bun and workerd outputs, and archive the declarations beside every build file
    expect([first.metadata, first.distribution.fileCount]).toEqual([
        {
            name: "@example/greeting",
            version: "2026.9.0",
            type: "module",
            exports: {
                ".": {
                    types: "./build/src/index.ts",
                    workerd: `./build/${workerd.exports["."]}`,
                    default: `./build/${bun.exports["."]}`,
                },
            },
            dependencies: { "@example/answer": "2026.9.0" },
            optionalDependencies: {},
            gitHead: COMMIT,
        },
        list.length + 2,
    ]);

    // produce byte-identical archives with the same description
    expect([
        second.manifest,
        second.distribution,
        await new Response(second.open()).bytes(),
    ]).toEqual([first.manifest, first.distribution, await new Response(first.open()).bytes()]);
});
