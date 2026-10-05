import { type Publisher } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { schema } from "@destack/schema";
import { Authentication } from "../authentication/index.ts";
import { defineService } from "../declare/service.ts";
import { ServiceMount } from "../service/mount.ts";
import { startTelemetry } from "@destack/telemetry/bun";
import { emptyBuild } from "../test/build.ts";
import { WorkloadRunner } from "./runner.ts";
import type { WorkloadStart } from "./start.ts";
import { defineWorkload, type CellDirectory } from "./workload.ts";
import { WEBHOOK_PATH } from "./start.ts";
import { defineTrigger, type WebhookParameters } from "../trigger/index.ts";
import { GitHubSignature } from "../github/index.ts";
import type { RunRequest } from "../trigger/index.ts";

/** The OTLP logs export the monitor receives, down to its resource and each record's event name. */
const LogsExport = schema
    .object({
        /** The logs by resource. */
        resourceLogs: schema
            .array(
                schema
                    .object({
                        /** The resource of the logs, read for its string attributes. */
                        resource: schema
                            .object({
                                attributes: schema.array(
                                    schema
                                        .object({
                                            key: schema.string(),
                                            value: schema
                                                .object({ stringValue: schema.string() })
                                                .loose(),
                                        })
                                        .loose(),
                                ),
                            })
                            .loose(),
                        /** The logs by instrumentation scope. */
                        scopeLogs: schema.array(
                            schema
                                .object({
                                    /** The records. */
                                    logRecords: schema.array(
                                        schema
                                            .object({
                                                /** The record's event name. */
                                                eventName: schema.string().exactOptional(),
                                            })
                                            .loose(),
                                    ),
                                })
                                .loose(),
                        ),
                    })
                    .loose(),
            )
            .exactOptional(),
    })
    .loose();

/** The runner's one service. */
const notes = defineService("notes", {});

/** The space the installation serves. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** The host's start message. */
const start: WorkloadStart = {
    callKey: "00".repeat(32),
    instance: schema.identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000002"),
    scope: spaceId,
    installation: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000003"),
    bindings: {},
    secret: "forwarding",
    egress: "http://host.test/.destack/egress",
    sampling: 1,
    manifest: "c".repeat(64),
};

test("serve a forwarded caller below the package's mount, and export telemetry naming its release through the host's egress with the runner's secret", async () => {
    // capture the telemetry the runner exports to the monitor
    const exported: {
        url: string;
        authorization: string | null;
        release: Record<string, string>[];
        events: string[];
    }[] = [];
    const fetch = globalThis.fetch;
    const exporting = async (input: RequestInfo | URL, options?: RequestInit) => {
        const request = new Request(input, options);
        const body = LogsExport.parse(await request.json());
        exported.push({
            url: request.url,
            authorization: request.headers.get("authorization"),
            release: (body.resourceLogs ?? []).map((group) =>
                Object.fromEntries(
                    group.resource.attributes
                        .filter(
                            ({ key }) => key.startsWith("service.") || key.startsWith("destack."),
                        )
                        .map(({ key, value }) => [key, value.stringValue]),
                ),
            ),
            events: (body.resourceLogs ?? []).flatMap((group) =>
                group.scopeLogs.flatMap((scope) =>
                    scope.logRecords.map((record) => record.eventName ?? ""),
                ),
            ),
        });

        return Response.json({});
    };

    // fake the runtime's fetch, which needs no connection ahead of a request
    globalThis.fetch = Object.assign(exporting, { preconnect: () => {} });
    onTestFinished(() => {
        globalThis.fetch = fetch;
    });

    // start a workload with a service that answers with the path and the caller
    const publisher: Publisher = {
        stream: () => {
            throw new Error("the fixture publisher streams no copies");
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
                                        caller: context.authentication?.claims.subject.id ?? null,
                                    }),
                            },
                        ],
                    }),
                },
                { package: notes.package },
            ),
            resources: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            publisher: () => publisher,
            directory: () => directory,
            runs: () => ({
                send: async () => {
                    throw new Error("the fixture records no runs");
                },
            }),
        },
        start,
        await emptyBuild(notes.package),
        startTelemetry,
        (error) => {
            throw error;
        },
    );

    // refuse a request without the host's secret, serve a forwarded caller, and refuse another mount
    const mount = `http://runner.test${ServiceMount.path(notes.package.id)}`;
    const unauthorized = await runner.fetch(new Request(`${mount}/notes`));
    const refusal: unknown = await unauthorized.json();
    const now = Date.now();
    const alice = principal.user.reference("universe", "alice");
    const headers = new Headers({ authorization: "Bearer forwarding" });
    new Authentication({
        subject: alice,
        subjects: [alice],
        credential: { kind: "session", id: "session" },
        audience: notes.package.id,
        scope: spaceId,
        verifiedAt: now,
        expiresAt: now + 60_000,
    }).forward(headers);
    const served: unknown = await (
        await runner.fetch(new Request(`${mount}/notes`, { headers }))
    ).json();
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
                release: [
                    {
                        "service.name": notes.package.name,
                        "service.version": notes.package.version,
                        "service.instance.id": start.instance,
                        "destack.build.manifest": "c".repeat(64),
                    },
                ],
                events: ["workload.started"],
            },
        ],
    });
});

test("verify a webhook trigger's deliveries with each route's secret and record each delivery's call once as a run", async () => {
    // start a workload receiving pushes, whose secrets its resources keep per repository
    const pushes = defineTrigger(
        {
            name: "pushes",
            on: {
                webhook: {
                    signature: new GitHubSignature(),
                    route: "/{repository}",
                    secret: async ({ repository }: WebhookParameters) => `secret-${repository}`,
                },
            },
            call: (delivery) => {
                // require the route's repository
                const repository = delivery.parameters["repository"];
                if (repository === undefined) {
                    throw new TypeError("a push delivery's route has a repository");
                }

                return {
                    method: "repository.push",
                    input: { repository, payload: delivery.payload },
                    release: "2026.9.0",
                };
            },
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
                        triggers: [pushes],
                    }),
                },
                { package: notes.package },
            ),
            resources: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            publisher: () => ({
                stream: () => {
                    throw new Error("the fixture publisher streams no copies");
                },
            }),
            directory: () => directory,
            runs: () => ({
                send: async (request) => {
                    recorded.push(request);
                },
            }),
        },
        start,
        await emptyBuild(notes.package),
        async () => ({ shutdown: async () => {} }),
        (error) => {
            throw error;
        },
    );
    onTestFinished(() => runner.close());

    // accept a signed delivery, and refuse a forged one, an unknown trigger and a missing secret
    const body = JSON.stringify({ ref: "refs/heads/main" });
    const signed = await new GitHubSignature().sign(
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

        const refused: unknown = response.status === 202 ? null : await response.json();

        return [response.status, refused];
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
        [
            404,
            {
                defined: false,
                code: "NOT_FOUND",
                status: 404,
                message: "no webhook trigger releases",
            },
        ],
        [
            401,
            { defined: false, code: "UNAUTHORIZED", status: 401, message: "invalid host secret" },
        ],
    ]);
    const signature = signed.get("x-hub-signature-256");
    if (signature === null) {
        throw new TypeError("a GitHub delivery carries a signature");
    }
    const digest = signature.slice("sha256=".length);
    expect(recorded).toEqual([
        {
            call: {
                method: "repository.push",
                input: { repository: "notes", payload: { ref: "refs/heads/main" } },
                release: "2026.9.0",
            },
            triggerName: "pushes",
            event: { delivery: { id: digest, path: "/notes" } },
        },
    ]);
});

/** The directory as the fixture's workloads see it: no homes, locales or addresses. */
const directory: CellDirectory = {
    isHome: async () => false,
    locale: async () => undefined,
    address: async () => {},
};
