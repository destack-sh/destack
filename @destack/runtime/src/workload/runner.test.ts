import { Package, type PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import { type Uplink } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { Caller, principal } from "@destack/access";
import { schema } from "@destack/schema";
import { PackageHandle } from "../declare/index.ts";
import { PackageMount } from "../runtime/mount.ts";
import { startTelemetry } from "@destack/telemetry/bun";
import { WorkloadRunner } from "./runner.ts";
import type { RequestContext } from "../server/index.ts";
import type { WorkloadStart } from "./start.ts";
import { defineWorkload, type EphemeralHost } from "./workload.ts";
import { FORWARDED_URL_HEADER, ROUTE_PATH, WEBHOOK_PATH } from "./start.ts";
import { SOCKET_HEADER, SOCKET_PATH } from "./socket.ts";
import { Clock } from "@destack/environment";

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

/** The ephemeral host of runners with workloads that have no ephemeral objects. */
const refusingEphemeral: EphemeralHost = {
    open: () => Promise.reject(new Error("the fixture runner stores no ephemeral objects")),
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
    root: "00".repeat(32),
    instance: schema.identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000002"),
    space: spaceId,
    installation: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000003"),
    name: "notes",
    bindings: {},
    settings: { [`${notes.id}/language`]: "de" },
    secret: "forwarding",
    egress: "http://host.test/.destack/egress",
    url: "https://notes--home.test",
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

    // start a workload with packages that answer with the path and the caller, reading its installation's settings
    let settings: unknown;
    const publisher: Uplink = {
        stream: () => {
            throw new Error("the fixture publisher streams no replicas");
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
                        settings = started.server?.settings;

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
                caller: Caller.of(alice),
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

test("forward a webhook delivery below the webhook path to the workload, refusing one without the host's secret", async () => {
    // start a workload answering each delivery with its path below the webhook path
    const runner = await WorkloadRunner.start(
        {
            clock: Clock.system,
            ephemeral: refusingEphemeral,
            workload: defineWorkload(
                {
                    name: "main",
                    start: async () => ({
                        packages: [{ package: notes, router: {} }],
                        webhooks: {
                            fetch: async (request: Request) =>
                                new Response(new URL(request.url).pathname, { status: 202 }),
                        },
                    }),
                },
                { package: NOTES },
            ),
            resources: {},
            publisher: () => ({
                stream: () => {
                    throw new Error("the fixture publisher streams no replicas");
                },
                receive: () =>
                    Promise.reject(new Error("the fixture publisher receives no changes")),
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

    // forward a delivery with the host's secret, and refuse one without it
    const post = async (headers: Record<string, string>) => {
        const response = await runner.fetch(
            new Request(`http://runner.test${WEBHOOK_PATH}/pushes/notes`, {
                method: "POST",
                headers,
            }),
        );

        return [response.status, await response.text()];
    };
    expect([await post({ authorization: "Bearer forwarding" }), await post({})]).toEqual([
        [202, "/pushes/notes"],
        [
            401,
            JSON.stringify({
                defined: true,
                code: "UNAUTHORIZED",
                status: 401,
                message: "invalid host secret",
            }),
        ],
    ]);
});

test("accept a hibernated socket, hand its messages and close to the workload, and send its answers in order through the host", async () => {
    // start a workload echoing each message and recording each close, its host in memory
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
                    throw new Error("the fixture publisher streams no replicas");
                },
                receive: () =>
                    Promise.reject(new Error("the fixture publisher receives no changes")),
            }),
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

    // hand the workload two messages and the close of a socket the host hibernates
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
                        throw new Error("the fixture publisher streams no replicas");
                    },
                    receive: () =>
                        Promise.reject(new Error("the fixture publisher receives no changes")),
                }),
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

    // route to a workload serving its own package and to one serving only another package
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
