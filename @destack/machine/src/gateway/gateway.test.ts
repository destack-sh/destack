import { Failure } from "@destack/error";
import { ConnectionKind } from "@destack/connection/declare";
import { EGRESS_PROVIDER } from "@destack/resource";
import { testSimulation } from "@destack/object/test";
import { MemoryBuild } from "@destack/package/test";
import { expect, refusal, test } from "@destack/test";
import { Authentication, Caller, CALLER_HEADER, principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { Identifier, type JsonValue, nth, present, schema } from "@destack/schema";
import { Exchange } from "@destack/access/token";
import {
    CallError,
    Egress,
    InstallationMount,
    PackageMount,
    SpaceMount,
    VERSION_HEADER,
} from "@destack/object";
import { PARTITION_HEADER, WEBHOOK_PATH } from "@destack/object/workload";
import { ResourceId } from "@destack/resource";
import { Scope } from "@destack/sync";
import objectPackage from "@destack/object/package";
import type { InstanceSpec, Runtime, WorkloadResource } from "@destack/stack/runtime";
import { ids, TestSpace } from "@destack/stack/test";
import { Traffic } from "./traffic.ts";
import { RecordingTokenExchange } from "./test/exchange.ts";
import { deployNotes, openGateway } from "./test/gateway.ts";

/** The deployment of notes' earlier release. */
const EARLIER = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000c1"),
    release: "2026.9.0",
    status: "draining",
} as const;

/** The deployment of notes serving only releases since its release. */
const NEWEST = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000c3"),
    release: "2026.10.0",
    since: "2026.10.0",
    status: "active",
} as const;

/** The package of the space's own packages another origin is called at. */
const REMOTE_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a5");

/** The connections a workload binds: one applies a header, one a client certificate. */
const TOKEN_CONNECTION = schema
    .identifier("connection")
    .parse("connection-01996ab0-0000-7000-8000-0000000000d1");

/** The connection lending a client certificate. */
const CERTIFICATE_CONNECTION = schema
    .identifier("connection")
    .parse("connection-01996ab0-0000-7000-8000-0000000000d2");

/** The egress the calling instance sends its calls below. */
const EGRESS = "http://127.0.0.1:7470/.destack/egress";

/** A resource of the calling workload, such as a binding of an installation. */
function resource(
    id: string,
    fields: Pick<WorkloadResource, "kind" | "provider" | "reference" | "spec">,
): WorkloadResource {
    return {
        id: ResourceId.parse(id),
        scope: ids.space,
        packageId: ids.package,
        name: id,
        ...fields,
    };
}

/** The calling workload: a tasks instance in the space, bound to notes and to two lent APIs. */
const tasks: InstanceSpec = {
    settings: {},
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000b1"),
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-0000000000b2"),
    name: "tasks",
    scope: ids.space,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000b3"),
    build: await MemoryBuild.write(
        new Map(),
        {
            role: "app",
            package: {
                id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a7"),
                name: "@example/tasks",
                version: "2026.9.0",
            },
        },
        testSimulation().environment("fixture").crypto,
    ),
    output: "bun",
    workload: "main",
    enabled: [],
    sandbox: {},
    directories: [],
    served: [],
    installations: [],
    resources: [
        resource("installation-01996ab0-0000-7000-8000-0000000000e1", {
            kind: "installation",
            provider: "http",
            reference: "notes",
            spec: { package: ids.package },
        }),
        resource(TOKEN_CONNECTION, {
            kind: ConnectionKind.name,
            provider: EGRESS_PROVIDER,
            reference: TOKEN_CONNECTION,
            spec: { resource: "https://api.example.com/v1", scopes: [] },
        }),
        resource(CERTIFICATE_CONNECTION, {
            kind: ConnectionKind.name,
            provider: EGRESS_PROVIDER,
            reference: CERTIFICATE_CONNECTION,
            spec: { resource: "https://api.example.com/v1", scopes: [] },
        }),
    ],
};

/** A runtime recording the calls it forwards, knowing the calling instance by its secret. */
class RecordingRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "bun";
    /** The forwarded calls: instance, path, caller subject and authorization. */
    readonly forwarded: [string, string, string, string | null][] = [];
    /** The forwarded authentications. */
    readonly authentications: Authentication[] = [];
    /** The delivered webhook requests: instance, path and authorization. */
    readonly received: [string, string, string | null][] = [];

    /** Start nothing. */
    async start(): Promise<void> {}

    /** Stop nothing. */
    async stop(): Promise<void> {}

    /** Report every instance running. */
    isRunning(): boolean {
        return true;
    }

    /** Find the calling instance by its secret. */
    identify(secret: string): InstanceSpec | undefined {
        return secret === "tasks-secret" ? tasks : undefined;
    }

    /** Record a forwarded call and answer the instance it reached. */
    async fetch(
        instanceId: string,
        path: string,
        request: Request,
        authentication: Authentication | undefined,
    ): Promise<Response> {
        const caller = present(authentication, "the forwarded call's caller");
        this.authentications.push(caller);
        this.forwarded.push([
            instanceId,
            path,
            caller.caller.subject.id,
            request.headers.get("authorization"),
        ]);

        return Response.json({ instanceId });
    }

    /** Record a delivered webhook request. */
    async deliver(instanceId: string, path: string, request: Request): Promise<Response> {
        this.received.push([instanceId, path, request.headers.get("authorization")]);

        return new Response(null, { status: 202 });
    }

    /** Refuse a call of a build's export, which the gateway never makes. */
    async call(): Promise<JsonValue> {
        throw new Error("the gateway calls no export");
    }
}

/** Serve the fixture space's gateway over notes' deployments, recording what leaves it. */
async function openNotes(
    deployments: Parameters<typeof deployNotes>[1] = [EARLIER, NEWEST],
    options: Parameters<typeof openGateway>[1] = {},
) {
    const space = await TestSpace.open();
    await deployNotes(space, deployments);
    const runtime = new RecordingRuntime();
    const tokens = new RecordingTokenExchange(space.machine.environment);
    const gateway = openGateway(space, { runtimes: [runtime], tokens, ...options });

    return { space, runtime, tokens, gateway };
}

/** A person's verified call toward notes, as a page's origin hands it. */
function personAt(now: number): Authentication {
    return {
        caller: Caller.of(principal.user.reference(Scope.universe.id, "user-owner")),
        credential: {
            kind: "session",
            id: "fixture",
            audience: ids.package,
            verifiedAt: now,
            expiresAt: now + 60_000,
        },
    };
}

/** Read a gateway's answer: its JSON once it succeeds, else its refusal. */
async function answerOf(response: Response): Promise<unknown> {
    if (response.ok) {
        return response.json();
    }
    const failure = await Failure.read(response);

    return [failure.code, failure.reason, failure.message].join(": ");
}

/** Call an address below the egress as the calling instance, with forged identity headers. */
function egress(
    address: string,
    path: string,
    init: Omit<RequestInit, "headers"> & { readonly headers?: Record<string, string> } = {},
    secret = "tasks-secret",
) {
    return new Request(`${Egress.url(EGRESS, address)}${path}`, {
        ...init,
        headers: {
            authorization: `Bearer ${secret}`,
            [VERSION_HEADER]: "2026.10.0",
            [CALLER_HEADER]: "forged",
            cookie: "session=forged",
            ...init.headers,
        },
    });
}

/** Make a webhook request below the notes installation's mount. */
function webhook(): Request {
    return new Request(
        `${InstallationMount.url("http://127.0.0.1:7470", ids.space, ids.notes)}${WEBHOOK_PATH}/pushes/notes`,
        { method: "POST", body: "{}" },
    );
}

/** Answer that the called space moved to another machine. */
function movedAnswer(): Response {
    const misdirected = new CallError("MISDIRECTED_REQUEST", {
        reason: "SCOPE_MOVED",
        domain: "@destack/stack",
        message: `${ids.space} moved`,
        data: { scope: ids.space, machine: ids.machine },
    });

    return Response.json(misdirected.toJSON(), { status: misdirected.status });
}

test("route each call to the newest deployment serving the caller's release, and refuse other releases", async () => {
    // call notes as a person speaking each release
    const { gateway, space } = await openNotes();
    const authentication = personAt(space.machine.clock.now());
    const call = async (version?: string) => {
        const forwarded = gateway.ingress(
            ids.notes,
            "/notes/list",
            new Request("http://notes.test/", {
                headers: version === undefined ? {} : { [VERSION_HEADER]: version },
            }),
            authentication,
        );
        const refused = await refusal(forwarded);

        return refused === "done"
            ? schema
                  .looseObject({ instanceId: schema.string() })
                  .parse(await (await forwarded).json()).instanceId
            : refused.join(": ");
    };

    // serve old callers on the earlier deployment and current ones on the newest, a dev build as its release
    expect([
        await call("2026.8.0"),
        await call("2026.9.0"),
        await call("2026.10.0"),
        await call("2026.10.0-dev.2"),
        await call("2026.11.0"),
        await call("2026.11.0-dev.1"),
        await call(),
        await call("next"),
    ]).toEqual([
        EARLIER.instanceId,
        EARLIER.instanceId,
        NEWEST.instanceId,
        NEWEST.instanceId,
        `SERVICE_UNAVAILABLE: RUNNING_DEPLOYMENT_MISSING: no running deployment of ${ids.notes} runs release 2026.11.0`,
        `SERVICE_UNAVAILABLE: RUNNING_DEPLOYMENT_MISSING: no running deployment of ${ids.notes} runs release 2026.11.0-dev.1`,
        "BAD_REQUEST: VERSION_INVALID: requires Destack-Version",
        "BAD_REQUEST: VERSION_INVALID: invalid Destack-Version: next",
    ]);
});

test("count each request an installation serves and the bytes its responses stream out, each counted in exactly one take", async () => {
    // serve two calls through the space's traffic
    const traffic = new Traffic();
    const { gateway, space } = await openNotes([EARLIER, NEWEST], { traffic });
    const call = () =>
        gateway.ingress(
            ids.notes,
            "/notes/list",
            new Request("http://notes.test/", { headers: { [VERSION_HEADER]: "2026.9.0" } }),
            personAt(space.machine.clock.now()),
        );
    const read = traffic.countResponse(ids.space, ids.notes, await call());
    const unread = traffic.countResponse(ids.space, ids.notes, await call());
    const body = await read.text();
    const first = traffic.take();
    const rest = await unread.text();
    const second = traffic.take();
    const third = traffic.take();

    // count the requests and each response's bytes, tracing no work
    const bytes = new TextEncoder().encode(body).byteLength;
    const counted = { scope: ids.space, installation: ids.notes, cpuMilliseconds: 0 };
    expect({ first, second, third, isSame: rest === body }).toEqual({
        first: [{ ...counted, requests: 2, bytes }],
        second: [{ ...counted, requests: 0, bytes }],
        third: [],
        isSame: true,
    });
});

test("send a workload's calls as its installation, signing only the calls that leave the machine", async () => {
    // serve the object package in the space and record the calls leaving the machine
    const served: [string, string, string | null, string | null][] = [];
    const sent: [string, string | null, string | null][] = [];
    const { gateway, runtime, space } = await openNotes([EARLIER, NEWEST], {
        packages: [objectPackage],
        call: async (_packageId, request, authentication) => {
            served.push([
                new URL(request.url).pathname,
                present(authentication, "the served call's caller").credential.audience,
                request.headers.get("authorization"),
                request.headers.get(CALLER_HEADER) ?? request.headers.get("cookie"),
            ]);

            return Response.json({});
        },
        sign: (claims) => `signed ${claims.sub} for ${String(claims.aud)}`,
        send: async (request) => {
            sent.push([
                request.url,
                request.headers.get("authorization"),
                request.headers.get(CALLER_HEADER) ?? request.headers.get("cookie"),
            ]);

            return Response.json({});
        },
    });
    const call = async (address: string, path: string, secret?: string) => {
        const forwarded = gateway.egress(egress(address, `${path}?limit=1`, {}, secret));
        const refused = await refusal(forwarded);

        return refused === "done" ? (await forwarded).status : refused.join(": ");
    };

    // call the bound notes, the served object package and the space's packages at its machine, and refuse an unknown secret
    const remote = `${PackageMount.path(REMOTE_PACKAGE)}/notes/list`;
    const statuses = [
        await call("notes", "/notes/list"),
        await call("@destack/object", "/notes/list"),
        await call(ids.space, remote),
        await call("notes", "/notes/list", "other"),
    ];
    const installation = principal.installation.reference(ids.space, tasks.installationId).id;
    const located = await space.machine.directory.locate(ids.space, space.machine.clock.now());
    expect({ statuses, forwarded: runtime.forwarded, served, sent }).toEqual({
        statuses: [200, 200, 200, "UNAUTHORIZED: INSTANCE_SECRET_INVALID: invalid instance secret"],
        forwarded: [
            [NEWEST.instanceId, `${PackageMount.path(ids.package)}/notes/list`, installation, null],
        ],
        served: [["/notes/list", objectPackage.id, null, null]],
        sent: [
            [
                `${SpaceMount.url(located.url, ids.space)}${remote}?limit=1`,
                `Bearer signed ${installation} for ${PackageId.resource(REMOTE_PACKAGE)}`,
                null,
            ],
        ],
    });
});

test("send a workload's call once more with its body where the machine it called answers that the space moved", async () => {
    // answer the first call that the space moved, and the second as served
    const sent: [string, string][] = [];
    const { gateway } = await openNotes([EARLIER, NEWEST], {
        sign: () => "signed",
        send: async (request) => {
            sent.push([new URL(request.url).pathname, await request.text()]);

            return sent.length === 1 ? movedAnswer() : Response.json({ created: true });
        },
    });

    // call the space's own package with a sized body
    const body = JSON.stringify({ title: "Launch" });
    const path = `${PackageMount.path(REMOTE_PACKAGE)}/notes/create`;
    const response = await gateway.egress(
        egress(ids.space, path, {
            method: "POST",
            body,
            headers: { "content-length": String(body.length) },
        }),
    );

    // resolve the address again and resend the whole body once
    const target = `${SpaceMount.path(ids.space)}${path}`;
    expect({ status: response.status, sent }).toEqual({
        status: 200,
        sent: [
            [target, body],
            [target, body],
        ],
    });
});

test("answer a moved space to a call streaming a body above 1 MiB or of unknown length, sending it once", async () => {
    // answer every call that the space moved
    const sent: [string, number][] = [];
    const { gateway } = await openNotes([EARLIER, NEWEST], {
        sign: () => "signed",
        send: async (request) => {
            sent.push([new URL(request.url).pathname, (await request.text()).length]);

            return movedAnswer();
        },
    });

    // call with a body above the resend bound and with a stream of unknown length
    const path = `${PackageMount.path(REMOTE_PACKAGE)}/notes/create`;
    const large = "x".repeat(1024 * 1024 + 1);
    const sized = await gateway.egress(
        egress(ids.space, path, {
            method: "POST",
            body: large,
            headers: { "content-length": String(large.length) },
        }),
    );
    const streamed = await gateway.egress(
        egress(ids.space, path, {
            method: "POST",
            body: new Blob([JSON.stringify({ title: "Launch" })]).stream(),
            duplex: "half",
        }),
    );

    // answer misdirected to the caller after one attempt with the whole body
    const target = `${SpaceMount.path(ids.space)}${path}`;
    expect({ statuses: [sized.status, streamed.status], sent }).toEqual({
        statuses: [421, 421],
        sent: [
            [target, large.length],
            [target, JSON.stringify({ title: "Launch" }).length],
        ],
    });
});

test("answer another origin's 421 the egress cannot read as a moved scope to the caller unchanged, sending the call once", async () => {
    // answer a 421 without a call error
    let sent = 0;
    const { gateway } = await openNotes([EARLIER, NEWEST], {
        sign: () => "signed",
        send: async () => {
            sent += 1;

            return new Response("misdirected request", { status: 421 });
        },
    });
    const response = await gateway.egress(
        egress(ids.space, `${PackageMount.path(REMOTE_PACKAGE)}/notes/create`, {
            method: "POST",
            body: "{}",
            headers: { "content-length": "2" },
        }),
    );

    expect({ answer: [response.status, await response.text()], sent }).toEqual({
        answer: [421, "misdirected request"],
        sent: 1,
    });
});

test("forward a webhook below an installation to its newest running deployment, and refuse one without a deployment", async () => {
    // deliver a webhook to notes over its two deployments, and over none
    const deployed = await openNotes();
    const accepted = await deployed.gateway.serve(webhook());
    const undeployed = await openNotes([]);
    const refused = await undeployed.gateway.serve(webhook());

    expect({
        accepted: [accepted.status, deployed.runtime.received],
        refused: await answerOf(refused),
    }).toEqual({
        accepted: [202, [[NEWEST.instanceId, `${WEBHOOK_PATH}/pushes/notes`, null]]],
        refused: `SERVICE_UNAVAILABLE: RUNNING_DEPLOYMENT_MISSING: no running deployment of ${ids.notes} receives webhooks`,
    });
});

test("forward a verified person's call to the installation as that person, exchanging no token", async () => {
    // call notes as a verified person
    const { gateway, runtime, tokens, space } = await openNotes();
    const authentication = personAt(space.machine.clock.now());
    await gateway.ingress(
        ids.notes,
        "/notes/list",
        new Request("http://notes.test/", { headers: { [VERSION_HEADER]: "2026.10.0" } }),
        authentication,
    );

    expect([nth(runtime.authentications, 0), tokens.exchanged]).toEqual([authentication, []]);
});

test("relay an instance's OTLP export for the spec its secret proves", async () => {
    // record each export the space observes
    const observed: [string, unknown][] = [];
    const { gateway } = await openNotes([EARLIER, NEWEST], {
        observe: async (signal, body) => {
            observed.push([signal, body]);
        },
    });
    const send = async (secret: string, path: string, body: string) =>
        answerOf(
            await gateway.serve(
                new Request(`http://127.0.0.1:7470${path}`, {
                    method: "POST",
                    headers: { authorization: `Bearer ${secret}` },
                    body,
                }),
            ),
        );

    // relay the export of the instance the secret proves
    expect([
        await send("tasks-secret", "/.destack/telemetry/v1/traces", '{"resourceSpans":[]}'),
        await send("other", "/.destack/telemetry/v1/traces", '{"resourceSpans":[]}'),
        await send("tasks-secret", "/.destack/telemetry/v1/profiles", "{}"),
        await send("tasks-secret", "/.destack/telemetry/v1/logs", "logs"),
        observed,
    ]).toEqual([
        {},
        "UNAUTHORIZED: INSTANCE_SECRET_INVALID: invalid instance secret",
        "NOT_FOUND: SIGNAL_UNKNOWN: no OTLP signal at this path",
        "BAD_REQUEST: BODY_NOT_JSON: an OTLP/JSON logs export is not JSON",
        [["traces", { resourceSpans: [] }]],
    ]);
});

test("apply a bound connection's credential to a workload's call to its API without its secret or identity", async () => {
    // borrow a header from one connection and a client certificate from the other
    const sent: [string, string | null, string | null, string | null, string | undefined][] = [];
    const { gateway } = await openNotes([EARLIER, NEWEST], {
        borrow: async (lent) => {
            const resource = "https://api.example.com/v1";
            if (lent.id === CERTIFICATE_CONNECTION) {
                const material = { kind: "mtls", certificate: "CERT", privateKey: "KEY" } as const;

                return { protocol: { kind: "mtls" }, credential: { material }, resource };
            }
            const protocol = { kind: "key", header: "authorization", scheme: "Bearer" } as const;

            return {
                protocol,
                credential: { material: { kind: "key", key: "lent-token" } },
                resource,
            };
        },
        send: async (request, options) => {
            sent.push([
                request.url,
                request.headers.get("authorization"),
                request.headers.get(CALLER_HEADER),
                request.headers.get("cookie"),
                options?.certificate?.certificate,
            ]);

            return Response.json({});
        },
    });

    // call both lent APIs as the instance with forged identity headers
    const call = async (id: string) => (await gateway.egress(egress(id, "/items?page=2"))).status;
    const statuses = [await call(TOKEN_CONNECTION), await call(CERTIFICATE_CONNECTION)];

    expect({ statuses, sent }).toEqual({
        statuses: [200, 200],
        sent: [
            ["https://api.example.com/v1/items?page=2", "Bearer lent-token", null, null, undefined],
            ["https://api.example.com/v1/items?page=2", null, null, null, "CERT"],
        ],
    });
});

test("send a person's own tool's call through a connection below its API, borrowed as them, and refuse an anonymous one", async () => {
    // borrow a key lease as whoever calls, and record what leaves the machine
    const borrowed: (string | undefined)[] = [];
    const sent: [string, string | null][] = [];
    const { gateway, space } = await openNotes([EARLIER, NEWEST], {
        borrow: async (_lent, consumer) => {
            borrowed.push(consumer.id);
            const protocol = { kind: "key", header: "authorization", scheme: "Bearer" } as const;
            const material = { kind: "key", key: "person-key" } as const;

            return { protocol, credential: { material }, resource: "https://api.example.com/v1" };
        },
        send: async (request) => {
            sent.push([request.url, request.headers.get("authorization")]);

            return Response.json({});
        },
    });

    // call through the connection as the person, then anonymously
    const path = `/.destack/connection/${TOKEN_CONNECTION}/items?page=2`;
    const call = async (authentication?: Authentication) =>
        (await gateway.serve(new Request(`http://127.0.0.1:7470${path}`), authentication)).status;
    const statuses = [await call(personAt(space.machine.clock.now())), await call()];

    expect({ statuses, borrowed, sent }).toEqual({
        statuses: [200, 401],
        borrowed: ["user-owner"],
        sent: [["https://api.example.com/v1/items?page=2", "Bearer person-key"]],
    });
});

test("send an instance's call for its subject under their grant with the token the universe exchanged, and its own calls signed as the installation", async () => {
    // record each call leaving the machine
    const sent: [string, string | null, string | null][] = [];
    const { gateway, tokens, space } = await openNotes([EARLIER, NEWEST], {
        sign: () => "signed",
        send: async (request) => {
            sent.push([
                new URL(request.url).pathname,
                request.headers.get("authorization"),
                request.headers.get(Exchange.header),
            ]);

            return Response.json({});
        },
    });

    // call the space's package for a person under their grant, and as the installation
    const person = principal.user.reference(Scope.universe.id, "user-ada");
    const asked = {
        subject: person,
        grants: [{ id: Identifier.create("grant", space.machine.environment), scope: person.id }],
    };
    const path = `${PackageMount.path(REMOTE_PACKAGE)}/note/get`;
    await gateway.egress(egress(ids.space, path, { headers: Exchange.ask(asked) }));
    await gateway.egress(egress(ids.space, path));

    const target = `${SpaceMount.path(ids.space)}${path}`;
    expect({ exchanged: tokens.exchanged, sent }).toEqual({
        exchanged: [
            [
                principal.installation.reference(tasks.scope, tasks.installationId),
                asked,
                { audience: REMOTE_PACKAGE, scope: ids.space },
            ],
        ],
        sent: [
            [target, "Bearer exchanged-1", null],
            [target, "Bearer signed", null],
        ],
    });
});

test("send a call to the partition it addresses, and resend a call arriving at the wrong partition once to the partition its store points to", async () => {
    // serve a room's calls only in the room's partition, recording each call's partition and body
    const room = "room-01996ab0-0000-7000-8000-0000000000c1";
    const reached: [string, string][] = [];
    const runtime: Runtime = {
        name: "bun",
        start: async () => {},
        stop: async () => {},
        isRunning: () => true,
        identify: () => undefined,
        deliver: async () => new Response(null, { status: 404 }),
        call: () => Promise.reject(new Error("the fixture calls no export")),
        fetch: async (_instanceId, _path, request, _caller, partition) => {
            reached.push([partition ?? "root", await request.text()]);

            // answer in the room's partition, and point any other partition there
            const misdirected = new CallError("MISDIRECTED_REQUEST", {
                reason: "PARTITION_MISDIRECTED",
                domain: "@destack/object",
                message: `${room} is served by partition ${room}`,
                data: { scope: room, partition: room },
            });

            return partition === room
                ? Response.json({ posted: true })
                : Response.json(misdirected.toJSON(), { status: misdirected.status });
        },
    };
    const { gateway, space } = await openNotes([NEWEST], { runtimes: [runtime] });
    const post = async (body: string, partition?: string) => {
        const headers: Record<string, string> = {
            [VERSION_HEADER]: NEWEST.release,
            "content-length": String(new TextEncoder().encode(body).length),
        };
        if (partition !== undefined) {
            headers[PARTITION_HEADER] = partition;
        }
        const response = await gateway.ingress(
            ids.notes,
            "/rooms/post",
            new Request("http://notes.test/", { method: "POST", headers, body }),
            personAt(space.machine.clock.now()),
        );

        return response.status;
    };

    // resend a call naming no partition, and send one naming the room straight there
    expect([await post("hello"), await post("hi", room), reached]).toEqual([
        200,
        200,
        [
            ["root", "hello"],
            [room, "hello"],
            [room, "hi"],
        ],
    ]);
});

test("refuse a request to an installation origin the space has no installation of, answering its refusal", async () => {
    // ask for an origin whose installation name the space has none of
    const { gateway } = await openNotes();
    const answered = await gateway.serve(new Request("http://nots--space--acme.localhost/"));

    expect([answered.status, await answerOf(answered)]).toEqual([
        404,
        "NOT_FOUND: INSTALLATION_NOT_FOUND: space--acme has no installation nots",
    ]);
});
