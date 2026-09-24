import { expect, test } from "@destack/test";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Health } from "@destack/service/health";
import { ServiceError } from "@destack/service/error";
import { implementPreview } from "./preview.ts";
import { PreviewPool } from "../preview/index.ts";
import { Server } from "@destack/service/server";
import { hosting, createCaller } from "./tests/fixture.ts";
import { createClient } from "@destack/service/client";
import { preview } from "../service/preview.ts";
import { requests } from "../../tests/fixture/web/request.ts";
import { linkDependencies, readDependencies } from "../source/dependency.ts";

test("serve an application and invalidate its edited source", async () => {
    const fixture = new URL("../../tests/fixture/web/", import.meta.url);
    const dependencies = await readDependencies(fileURLToPath(new URL("source/", fixture)));
    const directory = await mkdtemp(join(tmpdir(), "destack-local-"));
    const alice = createCaller("alice");
    try {
        await cp(new URL("source/", fixture), directory, {
            recursive: true,
            filter: (path) => !path.endsWith("/node_modules") && !path.endsWith("\\node_modules"),
        });
        await linkDependencies(fileURLToPath(new URL("source/", fixture)), directory);
        // retain real source and routing through the service lifecycle
        let released = 0;
        let shouldFailRelease = false;
        await using previews = new PreviewPool(
            {
                async open(owner, request) {
                    expect({ owner, request }).toEqual({
                        owner: alice.id,
                        request: { source: "web", application: "server" },
                    });

                    return {
                        options: {
                            directory,
                            dependencies,
                            application: requests.server,
                            server: { port: 0 },
                        },
                        async expose(server) {
                            return server.vite.resolvedUrls!.local[0];
                        },
                        async [Symbol.asyncDispose]() {
                            if (shouldFailRelease) {
                                shouldFailRelease = false;
                                throw new ServiceError("CONFLICT", {
                                    message: "checkout is still retained",
                                });
                            }
                            released++;
                        },
                    };
                },
            },
            { concurrency: 1, capacity: 2, retention: 60_000 },
        );
        await using server = Server.start({
            ...hosting,
            router: { preview: implementPreview(previews) },
            health: new Health("build"),
            audit: async () => {},
            drainTimeout: 1000,
        });
        let owner = "alice";
        const client = createClient(
            { preview },
            {
                url: "https://build.local",
                headers: () => ({ authorization: owner }),
                fetch: (request) => server.fetch(request),
            },
        );

        // wait for the public URL and verify caller isolation
        const started = await client.preview.start({ source: "web", application: "server" });
        expect(started.status).toBe("starting");
        const stream = await client.preview.watch({ id: started.id });
        let running = started;
        for await (const value of stream) {
            running = value;
            if (value.status !== "starting") {
                break;
            }
        }
        if (running.status !== "running") {
            throw new Error(JSON.stringify(running));
        }
        expect(await client.preview.get({ id: started.id })).toEqual(running);
        await expect(
            client.preview.start({ source: "web", application: "server" }),
        ).rejects.toMatchObject({ code: "RATE_LIMITED" });
        owner = "bob";
        expect(await client.preview.list()).toEqual([]);
        await expect(client.preview.get({ id: started.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
        });
        owner = "alice";
        const url = running.url;
        const initial = await fetch(url, { headers: { accept: "text/html" } });
        expect(initial.status).toBe(200);
        const document = await initial.text();
        const workspace = fileURLToPath(new URL("../../../../", import.meta.url)).replace(
            /\/$/,
            "",
        );
        await expect(
            document.replaceAll(directory, "$SOURCE").replaceAll(workspace, "$WORKSPACE"),
        ).toMatchFileSnapshot(fileURLToPath(new URL("expected/local.html", fixture)));

        // observe the completed reload through the served document
        const path = join(directory, "src/app.tsx");
        const source = await readFile(path, "utf8");
        await writeFile(path, source.replace("Hello Destack", "Edited Destack"));
        await expect
            .poll(
                async () => {
                    const edited = await fetch(url, { headers: { accept: "text/html" } });
                    expect(edited.status).toBe(200);

                    return await edited.text();
                },
                { timeout: 3000 },
            )
            .toBe(document.replace("Hello Destack", "Edited Destack"));

        // retain failed cleanup for retry and count it against the active preview limit
        shouldFailRelease = true;
        await expect(client.preview.stop({ id: started.id })).rejects.toMatchObject({
            code: "CONFLICT",
        });
        expect(released).toBe(0);
        await expect(
            client.preview.start({ source: "web", application: "server" }),
        ).rejects.toMatchObject({ code: "RATE_LIMITED" });

        // acknowledge stop only after the listener and retained checkout are released
        const stopped = await client.preview.stop({ id: started.id });
        expect(stopped).toEqual({
            id: started.id,
            source: "web",
            application: "server",
            createdAt: started.createdAt,
            updatedAt: stopped.updatedAt,
            stoppedAt: stopped.updatedAt,
            status: "stopped",
        });
        expect(await client.preview.stop({ id: started.id })).toEqual(stopped);
        expect(released).toBe(1);
        await expect(fetch(url)).rejects.toThrow();

        // stop a new preview while its source or listener is still starting
        const pending = await client.preview.start({ source: "web", application: "server" });
        const cancelled = await client.preview.stop({ id: pending.id });
        expect(cancelled).toEqual({
            id: pending.id,
            source: "web",
            application: "server",
            createdAt: pending.createdAt,
            updatedAt: cancelled.updatedAt,
            stoppedAt: cancelled.updatedAt,
            status: "stopped",
        });
        expect(released).toBe(2);
    } finally {
        await rm(directory, { recursive: true });
    }
});
