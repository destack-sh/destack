import { expect, onTestFinished, test } from "@destack/test";
import { ResourceId } from "@destack/resource";
import { health } from "../health/index.ts";
import { VERSION_HEADER } from "../request/index.ts";
import { defineProcedure } from "../service/index.ts";
import { defineService } from "./service.ts";
import { defineServiceBinding } from "./binding.ts";

/** A service binding at a reference, as a host hands it to a workload. */
const bound = (reference: string, credential?: string) => ({
    resource: ResourceId.parse("resource-01996ab0-0000-7000-8000-000000000001"),
    kind: "service",
    provider: "http",
    reference,
    ...(credential === undefined ? {} : { credential }),
});

test("call a bound service at its reference with the credential the host lends, speaking the service's release", async () => {
    // answer health checks, recording each call's credential and release
    const calls: (string | null)[][] = [];
    const server = Bun.serve({
        port: 0,
        fetch: (request) => {
            calls.push([request.headers.get("authorization"), request.headers.get(VERSION_HEADER)]);

            return Response.json({ name: "notes", status: "ready" });
        },
    });
    onTestFinished(() => server.stop(true));

    // connect through the binding's HTTP connector and call the service
    const service = defineService("notes", { health });
    const notes = defineServiceBinding("notes", service);
    const url = `http://127.0.0.1:${server.port}`;
    const client = await notes.connectors.http.connect(bound(url, "secret"), notes);
    const answer = await client.health.check();
    expect([answer, calls, notes.kind, notes.spec]).toEqual([
        { name: "notes", status: "ready" },
        [["Bearer secret", service.package.version]],
        "service",
        { service: { packageId: notes.package.id, name: "notes" } },
    ]);
});

test("refuse connecting a service binding the host lends no credential", async () => {
    const notes = defineServiceBinding("notes", defineService("notes", { health }));

    await expect(notes.connectors.http.connect(bound("http://127.0.0.1"), notes)).rejects.toThrow(
        "service binding notes holds no credential",
    );
});

test("name a binding's calls by their paths in the service's router, and refuse a procedure the service does not route", () => {
    // declare calls of the service's health procedures and of another service's
    const service = defineService("notes", { health });
    const other = defineService("other", {
        ping: {
            send: defineProcedure({ authentication: "identity", permission: null, audit: false }),
        },
    });
    const notes = defineServiceBinding("notes", service, {
        calls: [service.router.health.check, service.router.health.watch],
    });
    const stray = () => defineServiceBinding("notes", service, { calls: [other.router.ping.send] });

    // keep the dotted paths in the binding's spec and refuse the stray procedure
    expect(notes.spec.calls).toEqual(["health.check", "health.watch"]);
    expect(stray).toThrow("service binding notes calls a procedure its service does not route");
});
