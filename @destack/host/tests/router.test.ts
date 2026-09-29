import { expect, test } from "@destack/test";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { identifier, Version } from "@destack/schema";
import { Egress } from "@destack/service";
import { Caller } from "@destack/service/authentication";
import { VERSION_HEADER } from "@destack/service/request";
import { Scope } from "@destack/sync";
import { type Endpoint, Router } from "../src/router/index.ts";
import type { InstanceSpec, Runtime } from "../src/runtime/index.ts";

/** The space the installations serve. */
const scope = identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a1");

/** Another space, holding a called installation on another origin. */
const acme = identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000a6");

/** The installation whose deployments serve calls. */
const notes = identifier("installation").parse("installation-01996ab0-0000-7000-8000-0000000000a2");

/** The package of the called installation. */
const NOTES_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a3");

/** The package of a service the host mounts. */
const AUDIT_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a4");

/** The package of a service another origin serves. */
const REMOTE_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000a5");

/** A running instance of the calling installation. */
const tasks: InstanceSpec = {
    instanceId: identifier("instance").parse("instance-01996ab0-0000-7000-8000-0000000000b1"),
    installationId: identifier("installation").parse(
        "installation-01996ab0-0000-7000-8000-0000000000b2",
    ),
    scope,
    deploymentId: identifier("deployment").parse("deployment-01996ab0-0000-7000-8000-0000000000b3"),
    build: {} as BuildReader,
    output: "bun",
    workload: "main",
    resources: [],
};

/** A runtime recording the calls it forwards, and holding the calling instance's secret. */
class RecordingRuntime implements Runtime {
    /** The server runtime. */
    readonly name = "bun";
    /** The forwarded calls: instance, path, caller subject and authorization. */
    readonly forwarded: [string, string, string, string | null][] = [];

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
        caller: Caller,
    ): Promise<Response> {
        this.forwarded.push([
            instanceId,
            path,
            caller.authentication.subject.id,
            request.headers.get("authorization"),
        ]);

        return Response.json({ instanceId });
    }
}

/** Two deployments of the called installation: the earlier release, and one serving only releases since its own. */
const endpoints: Endpoint[] = [
    {
        instanceId: identifier("instance").parse("instance-01996ab0-0000-7000-8000-0000000000c1"),
        deploymentId: identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-0000000000c2",
        ),
        runtime: "bun",
        release: Version.parse("2026.9.0"),
    },
    {
        instanceId: identifier("instance").parse("instance-01996ab0-0000-7000-8000-0000000000c3"),
        deploymentId: identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-0000000000c4",
        ),
        runtime: "bun",
        release: Version.parse("2026.10.0"),
        since: Version.parse("2026.10.0"),
    },
];

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
    const caller = new Caller({
        credential: { kind: "session", id: "fixture" },
        audience: NOTES_PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });
    const call = (version?: string) =>
        router
            .ingress(
                notes,
                "/notes/list",
                new Request("http://notes.test/", {
                    headers: version === undefined ? {} : { [VERSION_HEADER]: version },
                }),
                caller,
            )
            .then(
                async (response) => ((await response.json()) as { instanceId: string }).instanceId,
                (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
            );

    // serve old callers on the earlier deployment, current ones on the newest, and refuse the rest
    expect([
        await call("2026.8.0"),
        await call("2026.9.0"),
        await call("2026.10.0"),
        await call("2026.11.0"),
        await call(),
        await call("next"),
    ]).toEqual([
        endpoints[0]!.instanceId,
        endpoints[0]!.instanceId,
        endpoints[1]!.instanceId,
        `UNAVAILABLE: no running deployment of ${notes} serves release 2026.11.0`,
        "BAD_REQUEST: requires Destack-Version",
        "BAD_REQUEST: invalid Destack-Version: next",
    ]);
});

test("send a workload's calls to its addresses as its installation, scoped to the space each targets, signing only calls leaving the host", async () => {
    // resolve an installation on this host, a mounted service and another origin
    const runtime = new RecordingRuntime();
    const served: [string, string, string | null][] = [];
    const sent: [string, string | null][] = [];
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
                                    caller.authentication.audience,
                                    request.headers.get("authorization"),
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
            `signed ${caller.authentication.subject.id} for ${caller.authentication.audience} in ${caller.authentication.scope}`,
        fetch: async (request) => {
            sent.push([request.url, request.headers.get("authorization")]);

            return Response.json({});
        },
    });
    const egress = "http://127.0.0.1:7470/.destack/egress";
    const call = (address: string, secret: string) =>
        router
            .egress(
                new Request(`${Egress.url(egress, address)}/notes/list?limit=1`, {
                    headers: { authorization: `Bearer ${secret}`, [VERSION_HEADER]: "2026.10.0" },
                }),
            )
            .then(
                (response) => response.status,
                (error: { code: string; message: string }) => `${error.code}: ${error.message}`,
            );

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
        forwarded: [[endpoints[1]!.instanceId, "/notes/list", installation, null]],
        served: [["/notes/list", AUDIT_PACKAGE, null]],
        sent: [
            [
                "https://notes.work.acme.destack.space/.destack/service/notes/list?limit=1",
                `Bearer signed ${installation} for ${REMOTE_PACKAGE} in ${acme}`,
            ],
        ],
    });
});
