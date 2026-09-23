import { expect, test } from "@destack/test";
import { mkdtemp, rm, mkdir, writeFile, readFile, realpath } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { readEndpoint } from "../endpoint/index.ts";
import { LocalClient } from "../client/local.ts";
import { connect } from "../client/client.ts";
import { createAuditClient } from "@destack/audit/client";
import { DaemonStore } from "./store.ts";

test("serve authenticated local administration and audit history through graceful shutdown", async () => {
    // start the real executable with isolated persistent state
    const directory = await mkdtemp(join(tmpdir(), "destack-daemon-http-"));
    const local = new LocalClient(directory);
    expect(await local.inspect()).toEqual({ status: "uninitialized" });
    const child = Bun.spawn(
        [
            process.execPath,
            "run",
            "--no-env-file",
            fileURLToPath(new URL("../main.ts", import.meta.url)),
        ],
        {
            env: { ...process.env, DESTACK_DIRECTORY: directory },
            stdout: "ignore",
            stderr: "pipe",
            timeout: 8000,
            killSignal: "SIGKILL",
        },
    );

    const change = new AbortController();
    try {
        // wait for publication after migrations and HTTP readiness
        await expect
            .poll(() => local.status(), { timeout: 2000, interval: 10 })
            .toHaveProperty("host.hostId");
        const endpoint = await readEndpoint(directory);
        const origin = `http://127.0.0.1:${endpoint.port}`;
        const authorization = `Bearer ${endpoint.token}`;
        const client = await local.connect();
        const initial = await client.status();
        // keep declared unfinished operations explicit through the real authenticated transport
        await expect(client.login.list()).rejects.toMatchObject({ code: "NOT_IMPLEMENTED" });
        await expect(client.instance.list({ limit: 10 })).rejects.toMatchObject({
            code: "NOT_IMPLEMENTED",
        });
        await expect(client.preview.list({ limit: 10 })).rejects.toMatchObject({
            code: "NOT_IMPLEMENTED",
        });
        expect(await local.inspect()).toEqual({ status: "connected", daemon: initial });
        // reject missing credentials and browser-originated administration
        for (const headers of [{}, { authorization, origin: "https://foreign.test" }]) {
            const response = await fetch(`${origin}/status`, { headers });
            expect(response.status).toBe(401);
            await response.arrayBuffer();
        }

        // mutate local state and query its delivered audit record through the same host
        const renamed = await client.host.rename({ name: "local integration host" });
        expect(renamed).toEqual({ ...initial.host, name: "local integration host" });
        const anonymous = connect({ url: origin });
        await expect(
            anonymous.host.rename({ name: "unauthenticated change" }),
        ).rejects.toMatchObject({ code: "UNAUTHORIZED" });
        const audit = createAuditClient({ url: origin, headers: { authorization } });
        const query = { scope: { type: "host" as const, hostId: initial.host.hostId }, limit: 100 };
        await expect
            .poll(
                async () =>
                    (await audit.list(query)).items.filter(
                        (record) => record.event.action.name === "host.rename",
                    ).length,
                { timeout: 2000, interval: 10 },
            )
            .toBe(1);

        // retain the verified local authority and attribute rejected credentials anonymously
        const history = (await audit.list(query)).items.map((record) => record.event);
        const rename = history.find((event) => event.action.name === "host.rename")!;
        expect({
            actor: rename.context.actor,
            subject: rename.context.subject,
            delegation: rename.context.delegation,
        }).toEqual({
            actor: { type: "host", authority: initial.host.hostId, id: initial.host.hostId },
            subject: { type: "host", authority: initial.host.hostId, id: initial.host.hostId },
            delegation: [],
        });
        await expect
            .poll(
                async () =>
                    (await audit.list(query)).items
                        .filter(
                            ({ event }) =>
                                event.action.name === "service.invoke" &&
                                event.context.actor.type === "anonymous" &&
                                event.result.stage === "result" &&
                                event.targets.procedure?.id === "host.rename",
                        )
                        .map(({ event }) => ({
                            actor: event.context.actor,
                            subject: event.context.subject,
                            result: event.result,
                        })),
                { timeout: 2000, interval: 10 },
            )
            .toEqual([
                {
                    actor: { type: "anonymous" },
                    subject: undefined,
                    result: { stage: "result", outcome: "denied", errorCode: "UNAUTHORIZED" },
                },
            ]);
        await expect(audit.list({ ...query, scope: { type: "global" } })).rejects.toMatchObject({
            code: "FORBIDDEN",
        });

        // register ordinary Git edits without requiring a cloud repository identity
        const repository = join(directory, "repository");
        await mkdir(repository);
        const git = Bun.spawn(["/usr/bin/git", "init", repository], {
            stdout: "ignore",
            stderr: "pipe",
        });
        expect(await git.exited, await new Response(git.stderr).text()).toBe(0);
        await writeFile(join(repository, "note.txt"), "an ordinary local edit\n");
        const registration = await client.checkout.register({ directory: repository });
        expect(registration).toEqual({
            id: expect.stringMatching(/^checkout-/),
            repositoryId: null,
            directory: await realpath(repository),
            createdAt: expect.any(Number),
        });
        expect(await client.checkout.register({ directory: repository })).toEqual(registration);
        expect(await client.checkout.get({ checkoutId: registration.id })).toEqual(registration);
        expect(await client.checkout.list({ limit: 10 })).toEqual({
            items: [registration],
            cursor: null,
        });

        // unregister twice without deleting files or retaining a visible registration
        expect(await client.checkout.unregister({ checkoutId: registration.id })).toEqual({});
        expect(await client.checkout.unregister({ checkoutId: registration.id })).toEqual({});
        expect(await client.checkout.list({ limit: 10 })).toEqual({ items: [], cursor: null });
        expect(await readFile(join(repository, "note.txt"), "utf8")).toBe(
            "an ordinary local edit\n",
        );

        // finish the stop response before releasing the listener and database
        await local.stop();
        expect(await child.exited, await new Response(child.stderr).text()).toBe(0);
        expect(await local.inspect()).toEqual({ status: "disconnected" });
        const storage = await DaemonStore.open(directory);
        try {
            expect(await storage.host.get()).toEqual(renamed);
        } finally {
            await storage.close();
        }
    } finally {
        // terminate only this fixture's process if an assertion prevents graceful shutdown
        change.abort();
        child.kill();
        await child.exited;
        await rm(directory, { recursive: true, force: true });
    }
});
