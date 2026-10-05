import { expect, refusal, test } from "@destack/test";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { aligned, schema, Version } from "@destack/schema";
import { Egress } from "@destack/service";
import { Authentication, AUTHENTICATION_HEADER, Lending } from "@destack/service/authentication";
import { VERSION_HEADER } from "@destack/service/request";
import { Scope } from "@destack/sync";
import { type Endpoint, Router } from "../src/router/index.ts";
import type { InstanceSpec, Runtime } from "../src/runtime/index.ts";
import { memoryBuild } from "../src/test/build.ts";

/** The space the installations serve. */
const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** Another space, with a called installation on another origin. */
const acme = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a6");

/** The installation whose deployments serve calls. */
const notes = schema
    .identifier("installation")
    .parse("installation-01996ab0-0000-7000-8000-0000000000a2");

/** The package of the called installation. */
const NOTES_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a3");

/** The package of a service the host mounts. */
const AUDIT_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a4");

/** The package of a service another origin serves. */
const REMOTE_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a5");

/** A running instance of the calling installation. */
const tasks: InstanceSpec = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000b1"),
    installationId: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-0000000000b2"),
    scope,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000b3"),
    build: await memoryBuild(
        {
            id: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a7"),
            name: "@example/tasks",
            version: "2026.9.0",
        },
        {},
        new Map(),
    ),
    output: "bun",
    workload: "main",
    capabilities: {},
    directories: [],
    secrets: [],
    resources: [],
};

/** A runtime recording the calls it forwards, and keeping the calling instance's secret. */
class RecordingRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "bun";
    /** The forwarded calls: instance, path, caller subject and authorization. */
    readonly forwarded: [string, string, string, string | null][] = [];
    /** The forwarded callers. */
    readonly callers: Authentication[] = [];
    /** The forwarded webhook requests: instance, path and authorization. */
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

    /** Record a forwarded call. */
    async fetch(
        instanceId: string,
        path: string,
        request: Request,
        caller: Authentication,
    ): Promise<Response> {
        this.callers.push(caller);
        this.forwarded.push([
            instanceId,
            path,
            caller.claims.subject.id,
            request.headers.get("authorization"),
        ]);

        return Response.json({ instanceId });
    }

    /** Record a forwarded webhook request. */
    async receive(instanceId: string, path: string, request: Request): Promise<Response> {
        this.received.push([instanceId, path, request.headers.get("authorization")]);

        return new Response(null, { status: 202 });
    }
}

/** The deployment of the called installation's earlier release. */
const earlier: Endpoint = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000c1"),
    scope,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000c2"),
    runtime: "bun",
    release: Version.parse("2026.9.0"),
};

/** The deployment of the called installation serving only releases since its release. */
const newest: Endpoint = {
    instanceId: schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000c3"),
    scope,
    deploymentId: schema
        .identifier("deployment")
        .parse("deployment-01996ab0-0000-7000-8000-0000000000c4"),
    runtime: "bun",
    release: Version.parse("2026.10.0"),
    since: Version.parse("2026.10.0"),
};

/** The two deployments of the called installation. */
const endpoints: readonly Endpoint[] = [earlier, newest];

/** The answer of the recording runtime: the instance a call arrived at. */
const ForwardedCall = schema.looseObject({ instanceId: schema.string() });

/** Make a webhook request to the called installation. */
function webhookRequest(): Request {
    return new Request("https://notes.personal.acme.destack.space/.destack/webhook/pushes/notes", {
        method: "POST",
        body: "{}",
    });
}

test("route each call to the newest deployment serving the caller's release, and refuse other releases", async () => {
    // route over the two deployments
    const runtime = new RecordingRuntime();
    const router = new Router({
        runtimes: [runtime],
        routes: {
            endpoints: async () => endpoints,
            resolve: async () => ({ kind: "remote", scope, url: "", audience: NOTES_PACKAGE }),
        },
        sign: async () => "unused",
        fetch: async () => new Response(null, { status: 500 }),
    });
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const caller = new Authentication({
        credential: { kind: "session", id: "fixture" },
        audience: NOTES_PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });
    const call = async (version?: string) => {
        const forwarded = router.ingress(
            notes,
            "/notes/list",
            new Request("http://notes.test/", {
                headers: version === undefined ? {} : { [VERSION_HEADER]: version },
            }),
            caller,
        );
        const refused = await refusal(forwarded);

        return refused === "done"
            ? ForwardedCall.parse(await (await forwarded).json()).instanceId
            : refused.join(": ");
    };

    // serve old callers on the earlier deployment, current ones on the newest, and refuse the rest
    expect([
        await call("2026.8.0"),
        await call("2026.9.0"),
        await call("2026.10.0"),
        await call("2026.11.0"),
        await call(),
        await call("next"),
    ]).toEqual([
        earlier.instanceId,
        earlier.instanceId,
        newest.instanceId,
        `SERVICE_UNAVAILABLE: no running deployment of ${notes} serves release 2026.11.0`,
        "BAD_REQUEST: requires Destack-Version",
        "BAD_REQUEST: invalid Destack-Version: next",
    ]);
});

test("send a workload's calls to its addresses as its installation, scoped to the space each targets, signing only calls leaving the host and stripping the identity claims it sent", async () => {
    // resolve an installation on this host, a mounted service and another origin
    const runtime = new RecordingRuntime();
    const served: [string, string, string | null, string | null][] = [];
    const sent: [string, string | null, string | null][] = [];
    const router = new Router({
        runtimes: [runtime],
        routes: {
            endpoints: async () => endpoints,
            resolve: async (_run, address) =>
                address === "notes"
                    ? {
                          kind: "installation",
                          scope,
                          installationId: notes,
                          audience: NOTES_PACKAGE,
                      }
                    : address === "@destack/audit"
                      ? {
                            kind: "service",
                            audience: AUDIT_PACKAGE,
                            fetch: async (request, caller) => {
                                served.push([
                                    new URL(request.url).pathname,
                                    caller.claims.audience,
                                    request.headers.get("authorization"),
                                    request.headers.get(AUTHENTICATION_HEADER) ??
                                        request.headers.get("cookie"),
                                ]);

                                return Response.json({});
                            },
                        }
                      : {
                            kind: "remote",
                            scope: acme,
                            url: "https://notes.work.acme.destack.space/.destack/service",
                            audience: REMOTE_PACKAGE,
                        },
        },
        sign: async (caller) =>
            `signed ${caller.claims.subject.id} for ${caller.claims.audience} in ${caller.claims.scope}`,
        fetch: async (request) => {
            sent.push([
                request.url,
                request.headers.get("authorization"),
                request.headers.get(AUTHENTICATION_HEADER) ?? request.headers.get("cookie"),
            ]);

            return Response.json({});
        },
    });
    const egress = "http://127.0.0.1:7470/.destack/egress";
    const call = async (address: string, secret: string) => {
        const forwarded = router.egress(
            new Request(`${Egress.url(egress, address)}/notes/list?limit=1`, {
                headers: {
                    authorization: `Bearer ${secret}`,
                    [VERSION_HEADER]: "2026.10.0",
                    [AUTHENTICATION_HEADER]: "forged",
                    cookie: "session=forged",
                },
            }),
        );
        const refused = await refusal(forwarded);

        return refused === "done" ? (await forwarded).status : refused.join(": ");
    };

    // call each destination as the installation, and refuse an unknown secret
    const statuses = [
        await call("notes", "tasks-secret"),
        await call("@destack/audit", "tasks-secret"),
        await call("notes.work.acme", "tasks-secret"),
        await call("notes", "other"),
    ];
    const installation = principal.installation.reference(scope, tasks.installationId).id;
    expect({ statuses, forwarded: runtime.forwarded, served, sent }).toEqual({
        statuses: [200, 200, 200, "UNAUTHORIZED: invalid instance secret"],
        forwarded: [[newest.instanceId, "/notes/list", installation, null]],
        served: [["/notes/list", AUDIT_PACKAGE, null, null]],
        sent: [
            [
                "https://notes.work.acme.destack.space/.destack/service/notes/list?limit=1",
                `Bearer signed ${installation} for ${REMOTE_PACKAGE} in ${acme}`,
                null,
            ],
        ],
    });
});

/** Route a workload's calls to a peer at the cell it left, answering with a remote answer, and on this host once it moved here. */
function movingRouter(answer: () => Response) {
    // resolve the peer at the cell it left first, then on this host once it moved here
    const runtime = new RecordingRuntime();
    const resolutions: string[] = [];
    const sent: [string, string][] = [];
    const router = new Router({
        runtimes: [runtime],
        routes: {
            endpoints: async () => endpoints,
            resolve: async (_run, address) => {
                resolutions.push(address);

                return resolutions.length === 1
                    ? {
                          kind: "remote",
                          scope: acme,
                          url: `https://left.cell.test/installation/${notes}`,
                          audience: REMOTE_PACKAGE,
                      }
                    : {
                          kind: "installation",
                          scope: acme,
                          installationId: notes,
                          audience: NOTES_PACKAGE,
                      };
            },
        },
        sign: async () => "signed",
        fetch: async (request) => {
            sent.push([request.url, await request.text()]);

            return answer();
        },
    });

    return { runtime, router, resolutions, sent };
}

/** Answer that the peer's space moved to this host. */
function movedAnswer(): Response {
    return Response.json(
        {
            defined: true,
            code: "MOVED",
            status: 421,
            message: `${acme} moves to host-arrived`,
            data: { scope: acme, cell: "host-arrived" },
        },
        { status: 421 },
    );
}

/** Call the peer through the egress as the calling instance, with a body and its headers. */
function callPeer(router: Router, body: BodyInit, headers: Record<string, string> = {}) {
    return router.egress(
        new Request(
            `${Egress.url("http://127.0.0.1:7470/.destack/egress", `${notes}.${acme}`)}/notes/create`,
            {
                method: "POST",
                body,
                headers: {
                    authorization: "Bearer tasks-secret",
                    [VERSION_HEADER]: "2026.10.0",
                    ...headers,
                },
            },
        ),
    );
}

test("send a workload's call once more where the cell it reached answers that the space moved, resolving its address again with its body", async () => {
    // call the peer with a sized body, answered on this host after the move
    const { runtime, router, resolutions, sent } = movingRouter(movedAnswer);
    const body = JSON.stringify({ title: "Launch" });
    const response = await callPeer(router, body, { "content-length": String(body.length) });
    const installation = principal.installation.reference(scope, tasks.installationId).id;
    const peer = `${notes}.${acme}`;
    expect({
        status: response.status,
        resolutions,
        sent,
        forwarded: runtime.forwarded,
    }).toEqual({
        status: 200,
        resolutions: [peer, peer],
        sent: [[`https://left.cell.test/installation/${notes}/notes/create`, '{"title":"Launch"}']],
        forwarded: [[newest.instanceId, "/notes/create", installation, null]],
    });
});

test("answer a moved space to a call streaming a body above 1 MiB or of unknown length, sending it once", async () => {
    // call the peer with a body above the resend bound, and with a stream of unknown length
    const large = "x".repeat(1024 * 1024 + 1);
    const sized = movingRouter(movedAnswer);
    const answered = await callPeer(sized.router, large, {
        "content-length": String(large.length),
    });
    const unsized = movingRouter(movedAnswer);
    const streamed = await callPeer(
        unsized.router,
        new Blob([JSON.stringify({ title: "Launch" })]).stream(),
    );

    // answer MOVED to the caller after one attempt with the whole body
    const peer = `${notes}.${acme}`;
    const target = `https://left.cell.test/installation/${notes}/notes/create`;
    expect({
        sized: [answered.status, sized.resolutions, sized.sent, sized.runtime.forwarded],
        unsized: [streamed.status, unsized.resolutions, unsized.sent, unsized.runtime.forwarded],
    }).toEqual({
        sized: [421, [peer], [[target, large]], []],
        unsized: [421, [peer], [[target, '{"title":"Launch"}']], []],
    });
});

test("answer another origin's 421 the egress cannot read as MOVED to the caller unchanged, sending the call once", async () => {
    // call the peer, whose origin answers a plain-text 421
    const { runtime, router, resolutions, sent } = movingRouter(
        () => new Response("misdirected request", { status: 421 }),
    );
    const response = await callPeer(router, "{}", { "content-length": "2" });

    // keep the answer's status and body
    expect({
        answer: [response.status, await response.text()],
        resolutions: resolutions.length,
        sent: sent.length,
        forwarded: runtime.forwarded,
    }).toEqual({ answer: [421, "misdirected request"], resolutions: 1, sent: 1, forwarded: [] });
});

test("forward a webhook request to the newest running deployment, and refuse one without a deployment", async () => {
    // route over the two deployments, then over none
    const runtime = new RecordingRuntime();
    const route = (listed: readonly Endpoint[]) =>
        new Router({
            runtimes: [runtime],
            routes: {
                endpoints: async () => listed,
                resolve: async () => ({ kind: "remote", scope, url: "", audience: NOTES_PACKAGE }),
            },
            sign: async () => "unused",
            fetch: async () => new Response(null, { status: 500 }),
        });
    const installation = tasks.installationId;
    const accepted = await route(endpoints).receive(
        installation,
        "/pushes/notes",
        webhookRequest(),
    );

    expect([accepted.status, runtime.received]).toEqual([
        202,
        [[newest.instanceId, "/pushes/notes", null]],
    ]);
    await expect(
        route([]).receive(installation, "/pushes/notes", webhookRequest()),
    ).rejects.toMatchObject({
        code: "SERVICE_UNAVAILABLE",
        message: `no running deployment of ${installation} receives webhooks`,
    });
});

test("lend each caller's authority to the installation it calls, as a lending the called cell verifies", async () => {
    // route with the host's lending
    const lending = await Lending.generate();
    const runtime = new RecordingRuntime();
    const router = new Router({
        runtimes: [runtime],
        routes: {
            endpoints: async () => endpoints,
            resolve: async () => ({ kind: "remote", scope, url: "", audience: NOTES_PACKAGE }),
        },
        sign: async () => "unused",
        fetch: async () => new Response(null, { status: 500 }),
        lending,
    });
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const caller = new Authentication({
        credential: { kind: "session", id: "fixture" },
        audience: NOTES_PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });
    await router.ingress(
        notes,
        "/notes/list",
        new Request("http://notes.test/", { headers: { [VERSION_HEADER]: "2026.10.0" } }),
        caller,
    );

    // forward the caller with a lending of its authority to the installation in its space
    const forwarded = aligned(runtime.callers, 0).claims;
    if (forwarded.delegation === undefined) {
        throw new TypeError("the forwarded caller carries no lending");
    }
    const { expiresAt, ...claim } = await lending.verify(forwarded.delegation);
    expect([forwarded.audience, claim, expiresAt > Date.now()]).toEqual([
        NOTES_PACKAGE,
        {
            subject: owner,
            subjects: [owner],
            installation: principal.installation.reference(scope, notes),
            scope,
        },
        true,
    ]);
});
