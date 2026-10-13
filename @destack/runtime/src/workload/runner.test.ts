import { StandardSignature } from "../trigger/index.ts";
import { Package, type PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { type Uplink } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { Caller, principal } from "@destack/access";
import { schema, Clock } from "@destack/schema";
import { PackageHandle } from "../declare/index.ts";
import { PackageMount } from "../runtime/mount.ts";
import { startTelemetry } from "@destack/telemetry/bun";
import { WorkloadRunner } from "./runner.ts";
import type { RequestContext } from "../server/index.ts";
import type { WorkloadStart } from "./start.ts";
import { defineWorkload, type EphemeralHost, type InstallationDirectory } from "./workload.ts";
import { FORWARDED_URL_HEADER, ROUTE_PATH, WEBHOOK_PATH } from "./start.ts";
import { SOCKET_HEADER, SOCKET_PATH } from "./socket.ts";
import { defineTrigger, type WebhookParameters } from "../trigger/index.ts";
import type { RunRequest } from "../trigger/index.ts";

/** The package-qualified repository type the calls call. */
const REPOSITORY = {
    packageId: schema.identifier("package").parse("package-01996ab0-0000-7000-8000-000000000001"),
    type: "repository",
};

/** The OTLP logs export the observability service receives. */
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

/** The ephemeral host of runners whose workloads keep no ephemeral objects. */
const refusingEphemeral: EphemeralHost = {
    open: () => Promise.reject(new Error("the fixture runner keeps no ephemeral objects")),
    channel: () => {
        throw new Error("the fixture runner replicates no ephemeral objects");
    },
};

/** The runner's own package. */
const NOTES = Package.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000a1",
    name: "@acme/notes",
    version: "2026.9.0",
});

/** The handle of the runner's own package. */
const notes = PackageHandle.of(NOTES, {});

/** Another package the runner's workload also serves. */
const events = PackageHandle.of(
    Package.parse({
        id: "package-01996ab0-0000-7000-8000-0000000000e1",
        name: "@acme/events",
        version: "2026.10.0",
    }),
    {},
);

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
    settings: { [`${notes.id}/language`]: "de" },
    secret: "forwarding",
    egress: "http://host.test/.destack/egress",
    sampling: 1,
};

test("start the workload with its installation's settings, serve a forwarded caller below the mount of each package the workload serves, and export telemetry naming only its release to the host with the runner's secret", async () => {
    // capture the telemetry the runner exports to its host
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

    // fake the runtime's fetch
    globalThis.fetch = Object.assign(exporting, { preconnect: () => {} });
    onTestFinished(() => {
        globalThis.fetch = fetch;
    });

    // start a workload with packages that answer with the path and the caller, keeping its installation's settings
    let settings: unknown;
    const publisher: Uplink = {
        stream: () => {
            throw new Error("the fixture publisher streams no copies");
        },
        receive: () => Promise.reject(new Error("the fixture publisher receives no changes")),
    };
    const runner = await WorkloadRunner.start(
        {
            clock: Clock.system,
            ephemeral: refusingEphemeral,
            workload: defineWorkload(
                {
                    name: "main",
                    start: async (started) => {
                        settings = started.installation?.settings;

                        return {
                            packages: [notes, events].map((handle) => ({
                                package: handle,
                                router: {},
                                fetch: async (request: Request, context: RequestContext) =>
                                    Response.json({
                                        package: handle.name,
                                        path: new URL(request.url).pathname,
                                        caller: context.caller().subject?.id ?? null,
                                    }),
                            })),
                        };
                    },
                },
                { package: NOTES },
            ),
            resources: {},
            publisher: () => publisher,
            directory: () => directory,
            runs: () => ({
                send: async () => {
                    throw new Error("the fixture records no runs");
                },
            }),
        },
        start,
        await MemoryBuild.write(new Map(), { package: NOTES }),
        startTelemetry,
        (error) => {
            throw error;
        },
    );

    // refuse a request without the host's secret
    const mount = `http://runner.test${PackageMount.path(notes.id)}`;
    const unauthorized = await runner.fetch(new Request(`${mount}/notes`));
    const refusal: unknown = await unauthorized.json();
    const now = Date.now();
    const alice = principal.user.reference("universe", "alice");
    const forwarded = (audience: PackageId) => {
        const headers = new Headers({ authorization: "Bearer forwarding" });
        Caller.forward(
            {
                subject: alice,
                subjects: [alice],
                credential: {
                    kind: "session",
                    id: "session",
                    audience,
                    scope: spaceId,
                    verifiedAt: now,
                    expiresAt: now + 60_000,
                },
            },
            headers,
        );

        return headers;
    };
    const headers = forwarded(notes.id);
    const served: unknown = await (
        await runner.fetch(new Request(`${mount}/notes`, { headers }))
    ).json();
    const received: unknown = await (
        await runner.fetch(
            new Request(`http://runner.test${PackageMount.path(events.id)}/receive`, {
                headers: forwarded(events.id),
            }),
        )
    ).json();
    const other = await runner.fetch(new Request("http://runner.test/other/notes", { headers }));

    // close and export what remains
    await runner.close();

    expect({
        settings,
        unauthorized: [unauthorized.status, refusal],
        served,
        received,
        other: other.status,
        exported: exported.filter((request) => request.events.length > 0),
    }).toEqual({
        settings: { [`${notes.id}/language`]: "de" },
        unauthorized: [
            401,
            { defined: true, code: "UNAUTHORIZED", status: 401, message: "invalid host secret" },
        ],
        served: { package: notes.name, path: "/notes", caller: "alice" },
        received: { package: events.name, path: "/receive", caller: "alice" },
        other: 404,
        exported: [
            {
                url: "http://host.test/.destack/telemetry/v1/logs",
                authorization: "Bearer forwarding",
                release: [
                    {
                        "service.name": notes.name,
                        "service.version": notes.version,
                    },
                ],
                events: ["workload.started"],
            },
        ],
    });
});

test("verify a webhook trigger's deliveries with each route's secret and record each delivery's call once as a run", async () => {
    // start a workload receiving pushes
    const pushes = defineTrigger(
        {
            name: "pushes",
            on: {
                webhook: {
                    signature: new StandardSignature(),
                    path: "/{repository}",
                    secret: async ({ repository }: WebhookParameters) =>
                        `whsec_${btoa(`secret-${repository}`)}`,
                },
            },
            call: (delivery) => {
                // require the route's repository
                const repository = delivery.parameters["repository"];
                if (repository === undefined) {
                    throw new TypeError("a push delivery's route has a repository");
                }

                return {
                    type: REPOSITORY,
                    method: "push",
                    input: { repository, payload: delivery.payload },
                    release: "2026.9.0",
                };
            },
        },
        { package: NOTES },
    );
    const recorded: RunRequest[] = [];
    const runner = await WorkloadRunner.start(
        {
            clock: Clock.system,
            ephemeral: refusingEphemeral,
            workload: defineWorkload(
                {
                    name: "main",
                    start: async () => ({
                        packages: [{ package: notes, router: {} }],
                        triggers: [pushes],
                    }),
                },
                { package: NOTES },
            ),
            resources: {},
            publisher: () => ({
                stream: () => {
                    throw new Error("the fixture publisher streams no copies");
                },
                receive: () =>
                    Promise.reject(new Error("the fixture publisher receives no changes")),
            }),
            directory: () => directory,
            runs: () => ({
                send: async (request) => {
                    recorded.push(request);
                },
            }),
        },
        start,
        await MemoryBuild.write(new Map(), { package: NOTES }),
        async () => ({ shutdown: async () => {} }),
        (error) => {
            throw error;
        },
    );
    onTestFinished(() => runner.close());

    // accept a signed delivery and refuse the rest
    const body = JSON.stringify({ type: "push", ref: "refs/heads/main" });
    const signed = await new StandardSignature().sign(
        { id: "delivery-1", event: "push", body, sentAt: Date.now() },
        `whsec_${btoa("secret-notes")}`,
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
                defined: true,
                code: "UNAUTHORIZED",
                status: 401,
                message: "webhook signature does not match",
            },
        ],
        [
            404,
            {
                defined: true,
                code: "NOT_FOUND",
                status: 404,
                message: "no webhook trigger releases",
            },
        ],
        [401, { defined: true, code: "UNAUTHORIZED", status: 401, message: "invalid host secret" }],
    ]);
    expect(recorded).toEqual([
        {
            call: {
                type: REPOSITORY,
                method: "push",
                input: { repository: "notes", payload: { type: "push", ref: "refs/heads/main" } },
                release: "2026.9.0",
            },
            triggerName: "pushes",
            event: { delivery: { id: "delivery-1", path: "/notes" } },
        },
    ]);
});

/** The directory as the fixture's workloads see it. */
const directory: InstallationDirectory = {
    locale: async (subject) => {
        throw new Error(`no person ${subject} reads a locale here`);
    },
    messageKey: async (space) => {
        throw new Error(`no space ${space} is sent messages here`);
    },
};

test("accept a held socket, hand its messages and close to the workload, and send its answers in order through the host", async () => {
    // start a workload echoing each message and recording each close, its host kept in memory
    const sent: (readonly [string, string, string])[] = [];
    const closed: string[] = [];
    let accepted: Response | undefined;
    const host = {
        url: "https://host.test/.destack/socket",
        fetch: async (request: Request) => {
            sent.push([request.method, new URL(request.url).pathname, await request.text()]);

            return new Response(null, { status: 204 });
        },
    };
    const runner = await WorkloadRunner.start(
        {
            clock: Clock.system,
            ephemeral: refusingEphemeral,
            workload: defineWorkload(
                {
                    name: "main",
                    start: async (context) => {
                        accepted = context.hibernation?.accept({
                            id: "socket-1",
                            attachment: { room: "lobby" },
                        });

                        return {
                            packages: [{ package: notes, router: {} }],
                            sockets: {
                                message: async (socket, message) => {
                                    socket.send(
                                        new TextEncoder().encode(
                                            `echo ${new TextDecoder().decode(message)}`,
                                        ),
                                    );
                                },
                                close: async (socket) => {
                                    closed.push(socket.id);
                                },
                            },
                        };
                    },
                },
                { package: NOTES },
            ),
            resources: {},
            publisher: () => ({
                stream: () => {
                    throw new Error("the fixture publisher streams no copies");
                },
                receive: () =>
                    Promise.reject(new Error("the fixture publisher receives no changes")),
            }),
            directory: () => directory,
            runs: () => ({ send: async () => {} }),
            socketHost: host,
        },
        start,
        await MemoryBuild.write(new Map(), { package: NOTES }),
        async () => ({ shutdown: async () => {} }),
        (error) => {
            throw error;
        },
    );
    onTestFinished(() => runner.close());

    // hand the workload two messages and the close of a socket the host holds
    const event = (method: string, body?: string) =>
        runner.fetch(
            new Request(`http://runner.test${SOCKET_PATH}/socket-1`, {
                method,
                headers: {
                    authorization: "Bearer forwarding",
                    [SOCKET_HEADER]: JSON.stringify({
                        id: "socket-1",
                        attachment: { room: "lobby" },
                    }),
                },
                ...(body === undefined ? {} : { body }),
            }),
        );
    const statuses = [
        (await event("POST", "one")).status,
        (await event("POST", "two")).status,
        (await event("DELETE")).status,
    ];

    expect([accepted?.headers.get(SOCKET_HEADER), statuses, sent, closed]).toEqual([
        JSON.stringify({ id: "socket-1", attachment: { room: "lobby" } }),
        [204, 204, 204],
        [
            ["POST", "/.destack/socket/socket-1", "echo one"],
            ["POST", "/.destack/socket/socket-1", "echo two"],
        ],
        ["socket-1"],
    ]);
});

test("hand a routed request to the workload's own package at the route path, under the URL it asked for, and refuse a workload serving another package alone", async () => {
    // start a workload whose package answers the path and the URL a request asked for, then route a request to it
    const routed = async (served: PackageHandle) => {
        await using runner = await WorkloadRunner.start(
            {
                clock: Clock.system,
                ephemeral: refusingEphemeral,
                workload: defineWorkload(
                    {
                        name: "main",
                        start: async () => ({
                            packages: [
                                {
                                    package: served,
                                    router: {},
                                    fetch: async (request: Request) =>
                                        new Response(
                                            `${new URL(request.url).pathname} ${request.headers.get(FORWARDED_URL_HEADER)}`,
                                        ),
                                },
                            ],
                        }),
                    },
                    { package: NOTES },
                ),
                resources: {},
                publisher: () => ({
                    stream: () => {
                        throw new Error("the fixture publisher streams no copies");
                    },
                    receive: () =>
                        Promise.reject(new Error("the fixture publisher receives no changes")),
                }),
                directory: () => directory,
                runs: () => ({ send: async () => {} }),
            },
            start,
            await MemoryBuild.write(new Map(), { package: NOTES }),
            async () => ({ shutdown: async () => {} }),
            (error) => {
                throw error;
            },
        );
        const response = await runner.fetch(
            new Request(`http://runner.test${ROUTE_PATH}`, {
                headers: {
                    authorization: "Bearer forwarding",
                    [FORWARDED_URL_HEADER]: "https://notes.acme.com/blog?page=2",
                },
            }),
        );

        return [response.status, await response.text()];
    };

    // route to a workload serving its own package and to one serving another package alone
    expect([await routed(notes), await routed(events)]).toEqual([
        [200, `${ROUTE_PATH} https://notes.acme.com/blog?page=2`],
        [
            404,
            JSON.stringify({
                defined: true,
                code: "NOT_FOUND",
                status: 404,
                message: "no package serves routes here",
            }),
        ],
    ]);
});
