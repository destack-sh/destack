import { type ReplicaSource } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { identifier } from "@destack/schema";
import { Caller } from "../authentication/index.ts";
import { defineService } from "../declare/service.ts";
import { ServiceMount } from "../service/mount.ts";
import { startTelemetry } from "@destack/telemetry/host";
import { WorkloadRunner } from "./runner.ts";
import type { WorkloadStart } from "./start.ts";
import { defineWorkload } from "./workload.ts";
import { WEBHOOK_PATH } from "./start.ts";
import { defineWebhook, WEBHOOK_SIGNATURES } from "../webhook/index.ts";
import type { RunRequest } from "../trigger/index.ts";

/** The runner's one service. */
const notes = defineService("notes", {});

/** The space the installation serves. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The host's start message. */
const start: WorkloadStart = {
    journalKey: "00".repeat(32),
    instance: identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000002"),
    scope: spaceId,
    installation: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-000000000003",
    ),
    bindings: {},
    secret: "forwarding",
    egress: "http://host.test/.destack/egress",
    sampling: 1,
};

test("serve a forwarded caller below the package's mount, and export telemetry through the host's egress with the runner's secret", async () => {
    // capture the telemetry the runner exports to the monitor
    const exported: { url: string; authorization: string | null; events: string[] }[] = [];
    const fetch = globalThis.fetch;
    globalThis.fetch = (async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        const body = (await request.json()) as {
            resourceLogs?: { scopeLogs: { logRecords: { eventName?: string }[] }[] }[];
        };
        exported.push({
            url: request.url,
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

    // start a workload with a service that answers with the path and the caller
    const source: ReplicaSource = {
        stream: () => {
            throw new Error("the fixture source streams no copies");
        },
    };
    const runner = await WorkloadRunner.start(
        {
            workload: defineWorkload(
                {
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
                },
                { package: notes.package },
            ),
            resources: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            replicas: () => source,
            runs: () => ({
                send: async () => {
                    throw new Error("the fixture records no runs");
                },
            }),
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
    const alice = principal.user.reference("universe", "alice");
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

    // close, exporting what remains
    await runner.close();

    expect({
        unauthorized: [unauthorized.status, refusal],
        served,
        other: other.status,
        exported: exported.filter((request) => request.events.length > 0),
    }).toEqual({
        unauthorized: [
            401,
            { defined: false, code: "UNAUTHORIZED", status: 401, message: "invalid host secret" },
        ],
        served: { path: "/notes", caller: "alice" },
        other: 404,
        exported: [
            {
                url: "http://host.test/.destack/egress/@destack/monitor/v1/logs",
                authorization: "Bearer forwarding",
                events: ["workload.started"],
            },
        ],
    });
});

test("verify a webhook's deliveries with each route's secret and record each delivery's call once as a run", async () => {
    // start a workload receiving pushes, whose secrets its resources hold per repository
    const pushes = defineWebhook(
        {
            name: "pushes",
            verification: "github",
            route: "/{repository}",
            secret: async ({ repository }) => `secret-${repository}`,
            call: (delivery) => ({
                method: "repository.push",
                input: { repository: delivery.parameters.repository!, payload: delivery.payload },
                release: "2026.9.0",
            }),
        },
        { package: notes.package },
    );
    const recorded: RunRequest[] = [];
    const runner = await WorkloadRunner.start(
        {
            workload: defineWorkload(
                {
                    name: "main",
                    start: async () => ({
                        services: [{ service: notes, router: {} }],
                        webhooks: [pushes],
                    }),
                },
                { package: notes.package },
            ),
            resources: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            replicas: () => ({
                stream: () => {
                    throw new Error("the fixture source streams no copies");
                },
            }),
            runs: () => ({
                send: async (request) => {
                    recorded.push(request);
                },
            }),
        },
        start,
        async () => ({ shutdown: async () => {} }) as never,
        (error) => {
            throw error;
        },
    );
    onTestFinished(() => runner.close());

    // accept a delivery signed with its repository's secret, and refuse a forged one, an unknown webhook and a missing secret
    const body = JSON.stringify({ ref: "refs/heads/main" });
    const signed = await WEBHOOK_SIGNATURES.github.sign(
        { id: "delivery-1", event: "push", body, sentAt: Date.now() },
        "secret-notes",
    );
    const post = async (path: string, headers: Headers) => {
        const response = await runner.fetch(
            new Request(`http://runner.test${WEBHOOK_PATH}${path}`, {
                method: "POST",
                headers,
                body,
            }),
        );

        return [response.status, response.status === 202 ? null : await response.json()];
    };
    const authorized = new Headers(signed);
    authorized.set("authorization", "Bearer forwarding");

    expect([
        await post("/pushes/notes", authorized),
        await post("/pushes/other", authorized),
        await post("/releases/notes", authorized),
        await post("/pushes/notes", signed),
    ]).toEqual([
        [202, null],
        [
            401,
            {
                defined: false,
                code: "UNAUTHORIZED",
                status: 401,
                message: "webhook signature does not match",
            },
        ],
        [404, { defined: false, code: "NOT_FOUND", status: 404, message: "no webhook releases" }],
        [
            401,
            { defined: false, code: "UNAUTHORIZED", status: 401, message: "invalid host secret" },
        ],
    ]);
    const digest = signed.get("x-hub-signature-256")!.slice("sha256=".length);
    expect(recorded).toEqual([
        {
            cause: "webhook",
            call: {
                method: "repository.push",
                input: { repository: "notes", payload: { ref: "refs/heads/main" } },
                release: "2026.9.0",
            },
            packageId: notes.package.id,
            trigger: "pushes",
            deliveryId: `/notes ${digest}`,
        },
    ]);
});
