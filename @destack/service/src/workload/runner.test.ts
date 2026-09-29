import { expect, onTestFinished, test } from "@destack/test";
import { principal, type AccessRelay } from "@destack/access";
import { identifier } from "@destack/schema";
import { Caller } from "../authentication/index.ts";
import { defineService } from "../declare/service.ts";
import { ServiceMount } from "../service/mount.ts";
import { startTelemetry } from "@destack/telemetry/host";
import { WorkloadRunner } from "./runner.ts";
import type { WorkloadStart } from "./start.ts";
import { defineWorkload } from "./workload.ts";

/** The runner's one service. */
const notes = defineService("notes", {});

/** The space the installation serves. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The host's start message. */
const start: WorkloadStart = {
    instance: identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000002"),
    workload: "main",
    scope: spaceId,
    installation: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-000000000003",
    ),
    bindings: {},
    credential: "first",
    secret: "forwarding",
    audit: "http://holder.test/service/audit",
    monitor: "http://holder.test/service/monitor",
    space: "http://holder.test/service/space",
    sampling: 1,
};

test("serve a forwarded caller below the package's mount, and export telemetry with the renewed credential", async () => {
    // capture the telemetry the runner exports to the monitor
    const exported: { authorization: string | null; events: string[] }[] = [];
    const fetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        const body = (await request.json()) as {
            resourceLogs?: { scopeLogs: { logRecords: { eventName?: string }[] }[] }[];
        };
        exported.push({
            authorization: request.headers.get("authorization"),
            events: (body.resourceLogs ?? []).flatMap((group) =>
                group.scopeLogs.flatMap((scope) =>
                    scope.logRecords.map((record) => record.eventName ?? ""),
                ),
            ),
        });

        return Response.json({});
    }) as typeof fetch;
    onTestFinished(() => {
        globalThis.fetch = fetch;
    });

    // start a workload whose service answers with the path and the caller it sees
    const relay: AccessRelay = {
        scope: spaceId,
        watch: () => {
            throw new Error("the fixture relay streams no access");
        },
    };
    const runner = await WorkloadRunner.start(
        {
            package: notes.package,
            workloads: {
                main: defineWorkload({
                    name: "main",
                    start: async () => ({
                        services: [
                            {
                                service: notes,
                                router: {},
                                route: async (request, context) =>
                                    Response.json({
                                        path: new URL(request.url).pathname,
                                        caller: context.caller?.authentication.subject.id ?? null,
                                    }),
                            },
                        ],
                    }),
                }),
            },
            resources: {},
            providers: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            access: () => relay,
        },
        start,
        startTelemetry,
        (error) => {
            throw error;
        },
    );

    // refuse a request without the host's secret, serve a forwarded caller, and refuse another mount
    const mount = `http://runner.test${ServiceMount.path(notes.package.id)}`;
    const unauthorized = await runner.fetch(new Request(`${mount}/notes`));
    const refusal = await unauthorized.json();
    const now = Date.now();
    const alice = principal.user.reference("global", "alice");
    const headers = new Headers({ authorization: "Bearer forwarding" });
    new Caller({
        subject: alice,
        subjects: [alice],
        credential: { kind: "session", id: "session" },
        audience: notes.package.id,
        scope: spaceId,
        verifiedAt: now,
        expiresAt: now + 60_000,
    }).forward(headers);
    const served = await (await runner.fetch(new Request(`${mount}/notes`, { headers }))).json();
    const other = await runner.fetch(
        new Request("http://runner.test/service/other/notes", { headers }),
    );

    // renew the credential, then close, exporting what remains
    runner.renew({ credential: "second" });
    await runner.close();

    expect({
        unauthorized: [unauthorized.status, refusal],
        served,
        other: other.status,
        exported: exported.filter((request) => request.events.length > 0),
    }).toEqual({
        unauthorized: [401, { code: "UNAUTHORIZED", message: "invalid host secret" }],
        served: { path: "/notes", caller: "alice" },
        other: 404,
        exported: [{ authorization: "Bearer second", events: ["workload.started"] }],
    });
});
