import { mkdtemp, readFile, rm } from "node:fs/promises";
import { ResourceId } from "@destack/resource";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { CapabilityError, PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { ServiceError } from "@destack/service";
import { Authentication, AUTHENTICATION_HEADER } from "@destack/service/authentication";
import { Scope } from "@destack/sync";
import { BunRuntime } from "../src/bun/runtime.ts";
import { LocalRuntime } from "../src/test/runtime.ts";
import { defineService } from "@destack/service";
import { defineWorkload } from "@destack/service/workload";
import type { InstanceSpec } from "../src/runtime/index.ts";
import { memoryBuild } from "../src/test/build.ts";
import { testCallKey } from "@destack/service/test";

/** The package the runner's build belongs to. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e7");

/** The echo of a forwarded call: the runner's secret beside what the runner saw. */
const CallEcho = schema.looseObject({ secret: schema.string() });

/** The echo of a forwarded webhook request. */
const WebhookEcho = schema.looseObject({
    path: schema.string(),
    isForwarded: schema.boolean(),
    caller: schema.unknown(),
});

/** A workload runner serving its start and its forwarded requests back, and crashing on request. */
const ECHO_RUNNER = `
import { appendFileSync } from "node:fs";
appendFileSync(process.env.HOME + "/starts", "start\\n");
const lines = console[Symbol.asyncIterator]();
const start = JSON.parse((await lines.next()).value);
const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    fetch: async (request) => {
        if (new URL(request.url).pathname.endsWith("/crash")) {
            process.exit(3);
        }
        return Response.json({
            method: request.method,
            body: await request.text(),
            path: new URL(request.url).pathname,
            isForwarded: request.headers.get("authorization") === "Bearer " + start.secret,
            caller: JSON.parse(request.headers.get("x-destack-authentication") ?? "null")?.subject.id ?? null,
            scope: start.scope,
            installation: start.installation,
            bindings: start.bindings,
            egress: start.egress,
            secret: start.secret,
        });
    },
});
console.log(JSON.stringify({ port: server.port }));
`;

test("spec an instance as a Bun process: bind service addresses at the egress with its secret and space, serve forwarded callers and webhooks without forged callers, identify it by its secret, report a crash, and stop it", async () => {
    // spec instances below a scratch directory
    const directory = await mkdtemp(join(tmpdir(), "destack-runtime-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const runtime = new BunRuntime({
        callKey: testCallKey,
        directory,
        egress: "http://127.0.0.1:7470/.destack/egress",
        sampling: async () => 1,
        output: () => {},
    });
    onTestFinished(() => runtime.close());

    // spec a build with the runner in its server output
    const files = new Map([["bun/workload.js", new TextEncoder().encode(ECHO_RUNNER)]]);
    const build = await memoryBuild(
        { id: PACKAGE, name: "@example/notes", version: "2026.9.0" },
        {
            bun: {
                runtime: "bun",
                emit: true,
                workloads: {
                    notes: {
                        entrypoint: "./workload/notes",
                        services: [],
                        triggers: [],
                        resources: [],
                        secrets: [],
                        connections: [],
                        compute: {},
                        capabilities: {},
                    },
                },
                views: {},
                directory: "bun",
                exports: { "./workload/notes": "bun/workload.js" },
                dependencies: {},
            },
        },
        files,
    );
    const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e4");
    const instanceId = schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000e2");
    const exits: number[] = [];
    const spec: InstanceSpec = {
        instanceId,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000e1"),
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000e3"),
        build,
        output: "bun",
        workload: "notes",
        capabilities: {},
        directories: [],
        secrets: [],
        scope,
        resources: [
            {
                id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000e5"),
                scope,
                kind: "database",
                spec: {},
                reference: "file:///spaces/main.db",
                packageId: PACKAGE,
                name: "main",
                provider: "sqlite",
            },
            {
                id: ResourceId.parse("resource-01996ab0-0000-7000-8000-0000000000e9"),
                scope,
                kind: "service",
                spec: { service: { packageId: PACKAGE, name: "service" } },
                reference: "notes",
                packageId: PACKAGE,
                name: "notes",
                provider: "http",
            },
        ],
    };
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const caller = new Authentication({
        credential: { kind: "session", id: "fixture" },
        audience: PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });
    const echo = async (path: string) =>
        CallEcho.parse(
            await (
                await runtime.fetch(
                    instanceId,
                    path,
                    new Request(`http://notes.test${path}`, {
                        method: "POST",
                        body: '{"after":1}',
                    }),
                    caller,
                )
            ).json(),
        );

    // start twice at once and again, serving one process, and identify it by its runner's secret
    const exited = async (code: number) => {
        exits.push(code);
    };
    await Promise.all([runtime.start(spec, exited), runtime.start(spec, exited)]);
    await runtime.start(spec, exited);
    const starts = await readFile(join(directory, spec.installationId, "data", "starts"), "utf8");
    const { secret, ...answered } = await echo("/replica/stream");
    const identified = [runtime.identify(secret)?.instanceId, runtime.identify("other")];
    const webhook = WebhookEcho.parse(
        await (
            await runtime.receive(
                instanceId,
                "/pushes/notes",
                new Request("http://notes.test/.destack/webhook/pushes/notes", {
                    method: "POST",
                    headers: {
                        [AUTHENTICATION_HEADER]: JSON.stringify(caller),
                        cookie: "session=forged",
                    },
                    body: "{}",
                }),
            )
        ).json(),
    );

    // report a crash to the cell and serve nothing afterwards
    await expect(
        runtime.fetch(instanceId, "/crash", new Request("http://notes.test/crash"), caller),
    ).rejects.toThrow(TypeError);
    await expect.poll(() => exits, { interval: 5 }).toEqual([3]);
    const isRunning = runtime.isRunning(instanceId);

    // refuse a spec binding another package's resource
    const foreign = runtime.start(
        {
            ...spec,
            instanceId: schema
                .identifier("instance")
                .parse("instance-01996ab0-0000-7000-8000-0000000000e6"),
            resources: spec.resources.map((bound) => ({
                ...bound,
                packageId: PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e8"),
            })),
        },
        exited,
    );
    await expect(foreign).rejects.toEqual(
        new ServiceError("PRECONDITION_FAILED", {
            message:
                "resource resource-01996ab0-0000-7000-8000-0000000000e5 binds no provisioned resource of package-01996ab0-0000-7000-8000-0000000000e7",
        }),
    );
    expect({
        webhook: [webhook.path, webhook.isForwarded, webhook.caller],
        answered,
        identified,
        isRunning,
        starts,
    }).toEqual({
        webhook: ["/.destack/webhook/pushes/notes", true, null],
        answered: {
            method: "POST",
            body: '{"after":1}',
            path: `/service/${PACKAGE}/replica/stream`,
            isForwarded: true,
            caller: "user-owner",
            scope,
            installation: spec.installationId,
            bindings: {
                main: {
                    resource: "resource-01996ab0-0000-7000-8000-0000000000e5",
                    kind: "database",
                    provider: "sqlite",
                    reference: "file:///spaces/main.db",
                },
                notes: {
                    resource: "resource-01996ab0-0000-7000-8000-0000000000e9",
                    kind: "service",
                    provider: "http",
                    reference: "http://127.0.0.1:7470/.destack/egress/notes",
                    credential: secret,
                    scope,
                },
            },
            egress: "http://127.0.0.1:7470/.destack/egress",
        },
        identified: [instanceId, undefined],
        isRunning: false,
        starts: "start\n",
    });
});

/** A workload runner answering each request with the status of a fetch of the host it names. */
const FETCH_RUNNER = `
const lines = console[Symbol.asyncIterator]();
await lines.next();
const server = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    fetch: async (request) => {
        const target = new URL(request.url).searchParams.get("target");
        const response = await fetch(target, { signal: AbortSignal.timeout(2000) });
        return Response.json({ status: response.status, text: response.ok ? await response.text() : null });
    },
});
console.log(JSON.stringify({ port: server.port }));
`;

/** The answer of the fetching runner. */
const FetchEcho = schema.object({ status: schema.number(), text: schema.string().nullable() });

test("run a workload sandboxed: call the hosts its capabilities allow, see others denied by the proxy, and refuse any host before it starts", async () => {
    // serve two loopback hosts, of which the workload may call one
    const directory = await mkdtemp(join(tmpdir(), "destack-runtime-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const allowed = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("allowed"),
    });
    const denied = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("denied"),
    });
    onTestFinished(() => allowed.stop(true));
    onTestFinished(() => denied.stop(true));
    const runtime = new BunRuntime({
        callKey: testCallKey,
        directory,
        egress: "http://127.0.0.1:7470/.destack/egress",
        sampling: async () => 1,
        output: () => {},
    });
    onTestFinished(() => runtime.close());

    // spec a workload granted the one host
    const workload = {
        entrypoint: "./workload/fetch",
        services: [],
        triggers: [],
        resources: [],
        secrets: [],
        connections: [],
        compute: {},
        capabilities: {},
    };
    const build = await memoryBuild(
        { id: PACKAGE, name: "@example/fetch", version: "2026.10.0" },
        {
            bun: {
                runtime: "bun",
                emit: true,
                workloads: { fetch: workload },
                views: {},
                directory: "bun",
                exports: { "./workload/fetch": "bun/workload.js" },
                dependencies: {},
            },
        },
        new Map([["bun/workload.js", new TextEncoder().encode(FETCH_RUNNER)]]),
    );
    const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d4");
    const instanceId = schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000d2");
    const spec: InstanceSpec = {
        instanceId,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000d1"),
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000d3"),
        build,
        output: "bun",
        workload: "fetch",
        scope,
        capabilities: {
            network: { connect: [allowed.url.host], reason: "reads the allowed host" },
        },
        directories: [],
        secrets: [],
        resources: [],
    };
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const caller = new Authentication({
        credential: { kind: "session", id: "fixture" },
        audience: PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });

    // fetch both hosts from the workload
    await runtime.start(spec, async () => {});
    const call = async (target: URL) =>
        FetchEcho.parse(
            await (
                await runtime.fetch(
                    instanceId,
                    `/fetch?target=${encodeURIComponent(target.href)}`,
                    new Request("http://fetch.test/fetch"),
                    caller,
                )
            ).json(),
        );
    const answers = [await call(allowed.url), await call(denied.url)];

    // refuse a workload granted any host before it starts
    const anyHost = runtime.start(
        {
            ...spec,
            instanceId: schema
                .identifier("instance")
                .parse("instance-01996ab0-0000-7000-8000-0000000000d6"),
            capabilities: { network: { connect: ["*"], reason: "crawls" } },
        },
        async () => {},
    );
    await expect(anyHost).rejects.toEqual(
        new CapabilityError(
            "UNENFORCEABLE",
            "network",
            "connecting to any host is not enforced by this host's sandbox, which allows listed hosts only",
        ),
    );
    expect(answers).toEqual([
        { status: 200, text: "allowed" },
        { status: 403, text: null },
    ]);
});

test("start one in-process workload for two starts of an instance at once", async () => {
    // run a workload that counts its starts and serves its package's one service
    const service = defineService("notes", {});
    const starts: string[] = [];
    const runtime = new LocalRuntime({
        egress: "http://127.0.0.1:7470/.destack/egress",
        callKey: testCallKey,
        report: (error) => {
            throw error;
        },
        runner: () => ({
            workload: defineWorkload(
                {
                    name: "main",
                    start: async () => {
                        starts.push("start");

                        return { services: [{ service, router: {} }] };
                    },
                },
                { package: service.package },
            ),
            resources: {},
            history: () => ({ ingest: async () => ({ events: 0 }) }),
            publisher: () => ({
                stream: () => {
                    throw new Error("the fixture publisher streams no copies");
                },
                receive: () =>
                    Promise.reject(new Error("the fixture publisher receives no changes")),
            }),
            directory: () => ({
                isHome: async () => false,
                locale: async () => undefined,
                address: async () => {},
            }),
            runs: () => ({
                send: async () => {
                    throw new Error("the fixture records no runs");
                },
            }),
        }),
    });
    onTestFinished(() => runtime.close());

    // start the instance twice at once
    const instanceId = schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000f2");
    const spec: InstanceSpec = {
        instanceId,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000f1"),
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000f4"),
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000f3"),
        build: await memoryBuild(service.package, {}, new Map()),
        output: "bun",
        workload: "main",
        capabilities: {},
        directories: [],
        resources: [],
        secrets: [],
    };
    await Promise.all([runtime.start(spec), runtime.start(spec)]);

    expect([starts, runtime.isRunning(instanceId)]).toEqual([["start"], true]);
});
