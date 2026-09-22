import { expect, test } from "@destack/test";
import { Health } from "@destack/service/health";
import { ServiceError } from "@destack/service/error";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { BuildServer } from "../server/index.ts";
import { connect } from "../client/index.ts";
import { expectBuild } from "../../tests/fixture.ts";
import { request } from "../../tests/fixture/library/request.ts";
import { inspectPackage } from "../inspect/index.ts";

test("inspect and build authorized source through the service", async () => {
    const fixture = new URL("../../tests/fixture/library/", import.meta.url);
    const directory = fileURLToPath(new URL("source/", fixture));
    const released: string[] = [];
    const stored: string[] = [];

    // resolve client references before exposing source to the compiler
    await using server = new BuildServer({
        health: new Health("build"),
        authorize: async ({ context }) => {
            if (context.owner !== "alice" && context.owner !== "bob") {
                throw new ServiceError("UNAUTHORIZED");
            }
        },
        audit: async () => {},
        limits: {
            build: { concurrency: 1, capacity: 4, retention: 60_000, timeout: 10_000 },
            preview: { concurrency: 1, capacity: 4, retention: 60_000 },
        },
        builds: {
            async open(owner, selection) {
                expect({ owner, selection }).toEqual({
                    owner: "alice",
                    selection: { source: "library", outputs: ["library"] },
                });

                return {
                    options: { directory, ...request },
                    async [Symbol.asyncDispose]() {
                        released.push("build");
                    },
                };
            },
            async inspect(owner, selection) {
                expect({ owner, selection }).toEqual({
                    owner: "alice",
                    selection: { source: "library", output: "library" },
                });

                return {
                    directory,
                    target: "server",
                    runtime: "bun",
                    async [Symbol.asyncDispose]() {
                        released.push("inspect");
                    },
                };
            },
            async store(owner, build) {
                await expectBuild(build, new URL("expected/", fixture));
                stored.push(owner);

                return "https://build.local/download/library.tgz";
            },
        },
        previews: {
            async open() {
                throw new ServiceError("NOT_FOUND", { message: "source has no web application" });
            },
        },
    });
    let owner = "alice";
    const client = connect({
        url: "https://build.local",
        fetch: async (request) => {
            const response = await server.handler.handle(request, { context: { owner } });

            return response.matched ? response.response : new Response(null, { status: 404 });
        },
    });

    // compare inspection to the same document distributed by a production build
    const inspection = await client.inspect({ source: "library", output: "library" });
    const expected = JSON.parse(
        await readFile(new URL("expected/inspect/library/index.json", fixture), "utf8"),
    );
    const { schema: _schema, ...document } = inspection;
    expect(document).toEqual(expected);
    expect(released).toEqual(["inspect"]);

    // require local and remote inspection to return the same complete description
    const local = await inspectPackage({ directory, target: "server", runtime: "bun" });
    expect(local).toEqual(inspection);

    // wait for the complete build, including storage and source release
    const started = await client.build.start({ source: "library", outputs: ["library"] });
    const stream = await client.build.watch({ id: started.id });
    let completed = started;
    for await (const state of stream) {
        completed = state;
    }
    const manifest = JSON.parse(await readFile(new URL("expected/manifest.json", fixture), "utf8"));
    expect(completed).toEqual({
        id: started.id,
        createdAt: started.createdAt,
        updatedAt: completed.updatedAt,
        completedAt: completed.updatedAt,
        cancellationRequested: false,
        state: "succeeded",
        progress: { phase: "storing" },
        result: {
            source: "library",
            manifest,
            download: "https://build.local/download/library.tgz",
        },
    });
    expect(released).toEqual(["inspect", "build"]);
    expect(stored).toEqual(["alice"]);

    // keep retained results private and allow their owner to remove them
    owner = "bob";
    expect(await client.build.list()).toEqual([]);
    await expect(client.build.get({ id: started.id })).rejects.toMatchObject({ code: "NOT_FOUND" });
    owner = "alice";
    await client.build.delete({ id: started.id });
    expect(await client.build.list()).toEqual([]);
});
