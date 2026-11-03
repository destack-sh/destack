import type { InstanceExit } from "@destack/stack/runtime";
import { testHost } from "@destack/test/host";
import { MemoryBuild } from "@destack/package/test";
import { ResourceId } from "@destack/resource";
import { join } from "node:path";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Caller, CALLER_HEADER, principal } from "@destack/access";
import { PackageId } from "@destack/package";
import { schema } from "@destack/schema";
import { PackageMount } from "@destack/object";
import { Scope } from "@destack/sync";
import { BunRuntime } from "./runtime.ts";
import { Kernel } from "../computer/kernel.ts";
import { WEBHOOK_PATH } from "@destack/object/workload";
import type { InstanceSpec } from "@destack/stack/runtime";
import { testKeys, testSimulation } from "@destack/object/test";
import { testStore } from "@destack/build/test";

/** The package the runner's build belongs to. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e7");

/** When the fixture's sessions were verified: 16 October 2026, UTC. */
const VERIFIED_AT = Date.UTC(2026, 9, 16);

/** The mount of the package's service below the installation. */
const MOUNT = PackageMount.path(PACKAGE);

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
    unix: process.argv[2],
    fetch: async (request) => {
        if (new URL(request.url).pathname.endsWith("/crash")) {
            process.exit(3);
        }
        return Response.json({
            method: request.method,
            body: await request.text(),
            path: new URL(request.url).pathname,
            isForwarded: request.headers.get("authorization") === "Bearer " + start.secret,
            caller: JSON.parse(request.headers.get("${CALLER_HEADER}") ?? "null")?.caller.subject.id ?? null,
            scope: start.scope,
            installation: start.installation,
            bindings: start.bindings,
            egress: start.egress,
            secret: start.secret,
        });
    },
});
console.log(JSON.stringify({}));
`;

test("run an instance as a Bun process bound at the egress, serving forwarded callers and refusing forged ones", async () => {
    // spec instances below a scratch directory, keyed from a world of their own
    const world = testSimulation().environment("runtime");
    const directory = await world.files.makeTemporaryDirectory("destack-runtime-");
    onTestFinished(() => world.files.remove(directory, { recursive: true }));
    const packages = await testStore(world);
    onTestFinished(() => packages[Symbol.asyncDispose]());
    const runtime = new BunRuntime({
        isolation: "sandbox",
        kernel: new Kernel(new Map()),
        environment: world,
        identity: async () => testKeys(world),
        directory,
        packages,
        egress: () => "http://127.0.0.1:7470/.destack/egress",
        sampling: async () => 1,
        url: async () => "http://127.0.0.1:7470/installation",
        output: () => {},
        report: (error) => {
            throw error;
        },
    });
    onTestFinished(() => runtime.close());

    // spec a build with the runner in its server output
    const files = new Map([["bun/workload.js", new TextEncoder().encode(ECHO_RUNNER)]]);
    const build = await MemoryBuild.write(
        files,
        {
            package: { id: PACKAGE, name: "@example/notes", version: "2026.9.0" },
            outputs: {
                bun: {
                    runtime: "bun",
                    emit: true,
                    workloads: {
                        notes: {
                            entrypoint: "./workload/notes",
                            packages: [],
                            resources: [],
                            secrets: [],
                            compute: {},
                            sandbox: {},
                        },
                    },
                    views: {},
                    directory: "bun",
                    exports: { "./workload/notes": { path: "bun/workload.js", imports: [] } },
                    dependencies: {},
                    peers: {},
                },
            },
        },
        world.crypto,
    );
    const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e4");
    const instanceId = schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000e2");
    const board = "board";
    const boardSpace = schema
        .identifier("space")
        .parse("space-01996ab0-0000-7000-8000-0000000000eb");
    const exits: InstanceExit[] = [];
    const exit = Promise.withResolvers<void>();
    await packages.put(build);
    const spec: InstanceSpec = {
        settings: {},
        instanceId,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000e1"),
        name: "notes",
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000e3"),
        build,
        output: "bun",
        workload: "notes",
        enabled: [],
        sandbox: {},
        directories: [],
        served: [],
        installations: [
            {
                packageId: PACKAGE,
                name: "board",
                package: PACKAGE,
                installation: board,
                scope: boardSpace,
            },
        ],
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
                kind: "installation",
                spec: { package: PACKAGE },
                reference: "notes",
                packageId: PACKAGE,
                name: "notes",
                provider: "http",
            },
        ],
    };
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const authentication = {
        caller: Caller.of(owner),
        credential: {
            kind: "session",
            id: "fixture",
            audience: PACKAGE,
            verifiedAt: VERIFIED_AT,
            expiresAt: VERIFIED_AT + 60_000,
        },
    };
    const echo = async (path: string) =>
        CallEcho.parse(
            await (
                await runtime.fetch(
                    instanceId,
                    `${MOUNT}${path}`,
                    new Request(`http://notes.test${path}`, {
                        method: "POST",
                        body: '{"after":1}',
                    }),
                    authentication,
                )
            ).json(),
        );

    // start twice at once and again and identify the one process by its runner's secret
    const exited = async (ended: InstanceExit) => {
        exits.push(ended);
        exit.resolve();
    };
    await Promise.all([runtime.start(spec, exited), runtime.start(spec, exited)]);
    await runtime.start(spec, exited);
    const starts = await world.files.readTextFile(
        join(directory, spec.installationId, "data", "starts"),
    );
    const { secret, ...answered } = await echo("/store/stream");
    const identified = [runtime.identify(secret)?.instanceId, runtime.identify("other")];
    const webhook = WebhookEcho.parse(
        await (
            await runtime.deliver(
                instanceId,
                `${WEBHOOK_PATH}/pushes/notes`,
                new Request("http://notes.test/.destack/webhook/pushes/notes", {
                    method: "POST",
                    headers: {
                        [CALLER_HEADER]: JSON.stringify(authentication),
                        cookie: "session=forged",
                    },
                    body: "{}",
                }),
            )
        ).json(),
    );

    // report a crash to the machine and serve nothing afterwards
    await expect(
        runtime.fetch(
            instanceId,
            `${MOUNT}/crash`,
            new Request("http://notes.test/crash"),
            authentication,
        ),
    ).rejects.toBeInstanceOf(TypeError);
    await exit.promise;
    expect(exits).toEqual([{ code: 3 }]);
    const isRunning = runtime.isRunning(instanceId);

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
            path: `/${PACKAGE}/store/stream`,
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
                    kind: "installation",
                    provider: "http",
                    reference: "http://127.0.0.1:7470/.destack/egress/notes",
                    credential: secret,
                    scope,
                },
                board: {
                    resource: board,
                    kind: "installation",
                    provider: "http",
                    reference: `http://127.0.0.1:7470/.destack/egress/${board}--${boardSpace}${MOUNT}`,
                    credential: secret,
                    scope: boardSpace,
                },
            },
            egress: "http://127.0.0.1:7470/.destack/egress",
        },
        identified: [instanceId, undefined],
        isRunning: false,
        starts: "start\n",
    });
});

/** A workload runner answering each request with the status of a fetch of the URL it names. */
const FETCH_RUNNER = `
const lines = console[Symbol.asyncIterator]();
await lines.next();
const server = Bun.serve({
    unix: process.argv[2],
    fetch: async (request) => {
        const target = new URL(request.url).searchParams.get("target");
        const response = await fetch(target, { signal: AbortSignal.timeout(2000) });
        return Response.json({ status: response.status, text: response.ok ? await response.text() : null });
    },
});
console.log(JSON.stringify({}));
`;

/** The answer of the fetching runner. */
const FetchEcho = schema.object({ status: schema.number(), text: schema.string().nullable() });

test("run a workload sandboxed: call the hosts its capabilities allow, see others denied by the proxy, and refuse any host before it starts", async () => {
    // serve two loopback hosts, keyed from a world of their own
    const world = testSimulation().environment("runtime");
    const directory = await world.files.makeTemporaryDirectory("destack-runtime-");
    onTestFinished(() => world.files.remove(directory, { recursive: true }));
    const allowed = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("allowed"),
    });
    const denied = testHost().server.http({
        hostname: "127.0.0.1",
        port: 0,
        fetch: () => new Response("denied"),
    });
    onTestFinished(() => allowed.stop(true));
    onTestFinished(() => denied.stop(true));
    const packages = await testStore(world);
    onTestFinished(() => packages[Symbol.asyncDispose]());
    const runtime = new BunRuntime({
        isolation: "sandbox",
        kernel: new Kernel(new Map()),
        environment: world,
        identity: async () => testKeys(world),
        directory,
        packages,
        egress: () => "http://127.0.0.1:7470/.destack/egress",
        sampling: async () => 1,
        url: async () => "http://127.0.0.1:7470/installation",
        output: () => {},
        report: (error) => {
            throw error;
        },
    });
    onTestFinished(() => runtime.close());

    // spec a workload granted the one host
    const workload = {
        entrypoint: "./workload/fetch",
        packages: [],
        resources: [],
        secrets: [],
        compute: {},
        sandbox: {},
    };
    const build = await MemoryBuild.write(
        new Map([["bun/workload.js", new TextEncoder().encode(FETCH_RUNNER)]]),
        {
            package: { id: PACKAGE, name: "@example/fetch", version: "2026.10.0" },
            outputs: {
                bun: {
                    runtime: "bun",
                    emit: true,
                    workloads: { fetch: workload },
                    views: {},
                    directory: "bun",
                    exports: { "./workload/fetch": { path: "bun/workload.js", imports: [] } },
                    dependencies: {},
                    peers: {},
                },
            },
        },
        world.crypto,
    );
    const scope = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d4");
    const instanceId = schema
        .identifier("instance")
        .parse("instance-01996ab0-0000-7000-8000-0000000000d2");
    await packages.put(build);
    const spec: InstanceSpec = {
        settings: {},
        instanceId,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-0000000000d1"),
        name: "notes",
        deploymentId: schema
            .identifier("deployment")
            .parse("deployment-01996ab0-0000-7000-8000-0000000000d3"),
        build,
        output: "bun",
        workload: "fetch",
        scope,
        enabled: [],
        sandbox: {
            network: { connect: [allowed.url.host] },
        },
        directories: [],
        served: [],
        installations: [],
        resources: [],
    };
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const authentication = {
        caller: Caller.of(owner),
        credential: {
            kind: "session",
            id: "fixture",
            audience: PACKAGE,
            verifiedAt: VERIFIED_AT,
            expiresAt: VERIFIED_AT + 60_000,
        },
    };

    // fetch both hosts from the workload
    await runtime.start(spec, async () => {});
    const call = async (target: URL) =>
        FetchEcho.parse(
            await (
                await runtime.fetch(
                    instanceId,
                    `${MOUNT}/fetch?target=${encodeURIComponent(target.href)}`,
                    new Request("http://fetch.test/fetch"),
                    authentication,
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
            enabled: [],
            sandbox: { network: { connect: ["*"] } },
        },
        async () => {},
    );
    expect(await refusal(anyHost)).toEqual([
        "UNPROCESSABLE_CONTENT",
        "ABILITY_UNENFORCEABLE",
        "connecting to any host is not enforced by this machine's sandbox, which allows listed hosts only",
    ]);
    expect(answers).toEqual([
        { status: 200, text: "allowed" },
        { status: 403, text: null },
    ]);
});
