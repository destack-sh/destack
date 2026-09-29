import { expect, onTestFinished, test } from "@destack/test";
import { health } from "../health/index.ts";
import { defineService } from "./service.ts";
import { defineServiceBinding } from "./binding.ts";

test("call a bound service over HTTP as the workload, with its current credential", async () => {
    // answer health checks, recording each call's credential
    const credentials: (string | null)[] = [];
    const server = Bun.serve({
        port: 0,
        fetch: (request) => {
            credentials.push(request.headers.get("authorization"));

            return Response.json({ name: "notes", status: "ready" });
        },
    });
    onTestFinished(() => server.stop(true));

    // bind the service through its HTTP provider with a rotating credential
    const notes = defineServiceBinding("notes", defineService("notes", { health }));
    const credential = { current: "first" };
    const url = `http://127.0.0.1:${server.port}`;
    const provider = await notes.providers.http(new URL(url), {
        credential: () => credential.current,
    });
    const record = {
        id: "resource-x",
        scope: "space-x",
        kind: "service",
        spec: {},
        reference: url,
    };
    const client = await provider.connect(record as never, notes);

    // call before and after the credential rotates
    const first = await client.health.check();
    credential.current = "second";
    const second = await client.health.check();
    expect([first, second, credentials, notes.kind, notes.spec]).toEqual([
        { name: "notes", status: "ready" },
        { name: "notes", status: "ready" },
        ["Bearer first", "Bearer second"],
        "service",
        { service: { packageId: notes.package.id, name: "notes" } },
    ]);
});

test("refuse connecting a service binding that holds no endpoint", async () => {
    const notes = defineServiceBinding("notes", defineService("notes", { health }));
    const provider = await notes.providers.http(new URL("http://127.0.0.1"), {
        credential: () => "token",
    });
    const record = {
        id: "resource-x",
        scope: "space-x",
        kind: "service",
        spec: {},
        reference: null,
    };

    await expect(provider.connect(record as never, notes)).rejects.toThrow(
        "service binding notes holds no endpoint",
    );
});
