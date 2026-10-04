import { expect, test } from "@destack/test";
import { BuildReader } from "@destack/package/manifest";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { PackageArchive } from "../src/pack/index.ts";
import { COMMIT, fixtureBuild } from "./fixture/package.ts";

test("pack a build into identical archives declaring its compiled exports and exact dependencies", async () => {
    // pack one build twice
    const build = await fixtureBuild("greeting");
    const first = await PackageArchive.pack(build, COMMIT);
    const second = await PackageArchive.pack(build, COMMIT);
    const list = await build.reader.distributed();
    const server = build.manifest.outputs["server"];
    if (server === undefined) {
        throw new TypeError("greeting builds no server output");
    }

    // declare the source types and the compiled server output, and archive the declarations beside every build file
    expect([first.metadata, first.distribution.fileCount]).toEqual([
        {
            name: "@example/greeting",
            version: "2026.9.0",
            type: "module",
            exports: {
                ".": {
                    types: "./build/src/index.ts",
                    default: `./build/${server.exports["."]}`,
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

test("refuse packing a build whose destack.json declares no publication", async () => {
    // serve a build's destack.json without its publication
    const build = await fixtureBuild("answer");
    const { publication: _publication, ...definition } = schema
        .record(schema.string(), schema.json())
        .parse(JSON.parse(new TextDecoder().decode(await build.reader.load("destack.json"))));
    const bytes = new TextEncoder().encode(JSON.stringify(definition));
    const load = (path: string) =>
        path === "destack.json" ? Promise.resolve(bytes) : build.reader.load(path);

    // refuse it before archiving any file
    await expect(
        PackageArchive.pack(
            {
                manifest: build.manifest,
                reader: new BuildReader(build.manifest, load),
                open: (path, signal) => build.open(path, signal),
            },
            COMMIT,
        ),
    ).rejects.toEqual(
        new ServiceError("BAD_REQUEST", { message: "destack.json declares no publication" }),
    );
});
