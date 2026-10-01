import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { principal } from "@destack/access";
import { PackageId } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { identifier } from "@destack/schema";
import { ServiceError } from "@destack/service";
import { Caller } from "@destack/service/authentication";
import { Scope } from "@destack/sync";
import { BunRuntime } from "../src/runtime/bun.ts";
import type { InstanceSpec } from "../src/runtime/index.ts";
import { testJournalKey } from "@destack/service/test";

/** The package the runner's build belongs to. */
const PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000e7");

/** A workload runner serving its start and its forwarded requests back, and crashing on request. */
const ECHO_RUNNER = `
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
            caller: JSON.parse(request.headers.get("x-destack-caller") ?? "null")?.subject.id ?? null,
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

test("spec an instance as a Bun process: bind services at the egress with its secret, serve forwarded callers and webhooks, identify it by its secret, report a crash, and stop it", async () => {
    // spec instances below a scratch directory
    const directory = await mkdtemp(join(tmpdir(), "destack-runtime-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    const runtime = new BunRuntime({
        journalKey: testJournalKey,
        directory,
        egress: "http://127.0.0.1:7470/.destack/egress",
        sampling: async () => 1,
        output: () => {},
    });
    onTestFinished(() => runtime.close());

    // spec a build with the runner in its server output
    const files = new Map([["bun/workload.js", new TextEncoder().encode(ECHO_RUNNER)]]);
    const build = {
        manifest: {
            package: { id: PACKAGE },
            outputs: {
                bun: {
                    directory: "bun",
                    workloads: { notes: { entrypoint: "./workload/notes" } },
                    exports: { "./workload/notes": "bun/workload.js" },
                },
            },
        },
        inventory: async () => [...files.keys()].map((path) => ({ path })),
        load: async (path: string) => files.get(path)!,
    } as unknown as BuildReader;
    const scope = identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000e4");
    const instanceId = identifier("instance").parse(
        "instance-01996ab0-0000-7000-8000-0000000000e2",
    );
    const exits: number[] = [];
    const spec: InstanceSpec = {
        instanceId,
        installationId: identifier("installation").parse(
            "installation-01996ab0-0000-7000-8000-0000000000e1",
        ),
        deploymentId: identifier("deployment").parse(
            "deployment-01996ab0-0000-7000-8000-0000000000e3",
        ),
        build,
        output: "bun",
        workload: "notes",
        scope,
        resources: [
            {
                id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-0000000000e5"),
                scope,
                kind: "database",
                spec: {},
                reference: "file:///spaces/main.db",
                packageId: PACKAGE,
                name: "main",
                providerCode: "sqlite",
            },
            {
                id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-0000000000e9"),
                scope,
                kind: "service",
                spec: { service: { packageId: PACKAGE, name: "service" } },
                reference: "notes",
                packageId: PACKAGE,
                name: "notes",
                providerCode: "http",
            },
        ],
    };
    const owner = principal.user.reference(Scope.universe.id, "user-owner");
    const caller = new Caller({
        credential: { kind: "session", id: "fixture" },
        audience: PACKAGE,
        subject: owner,
        subjects: [owner],
        verifiedAt: Date.now(),
        expiresAt: Date.now() + 60_000,
    });
    const echo = async (path: string) =>
        (
            await runtime.fetch(
                instanceId,
                path,
                new Request(`http://notes.test${path}`, {
                    method: "POST",
                    body: '{"after":1}',
                }),
                caller,
            )
        ).json() as Promise<{ secret: string }>;

    // start twice, serving one process, and identify it by the secret its runner holds
    const exited = async (code: number) => {
        exits.push(code);
    };
    await runtime.start(spec, exited);
    await runtime.start(spec, exited);
    const { secret, ...answered } = await echo("/replica/sync");
    const identified = [runtime.identify(secret)?.instanceId, runtime.identify("other")];
    const webhook = (await (
        await runtime.receive(
            instanceId,
            "/pushes/notes",
            new Request("http://notes.test/.destack/webhook/pushes/notes", {
                method: "POST",
                body: "{}",
            }),
        )
    ).json()) as Readonly<Record<string, unknown>>;

    // report a crash to the holder and serve nothing afterwards
    await expect(
        runtime.fetch(instanceId, "/crash", new Request("http://notes.test/crash"), caller),
    ).rejects.toThrow(TypeError);
    await expect.poll(() => exits, { interval: 5 }).toEqual([3]);
    const isRunning = runtime.isRunning(instanceId);

    // refuse a spec binding another package's resource
    const foreign = runtime.start(
        {
            ...spec,
            instanceId: identifier("instance").parse(
                "instance-01996ab0-0000-7000-8000-0000000000e6",
            ),
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
    }).toEqual({
        webhook: ["/.destack/webhook/pushes/notes", true, null],
        answered: {
            method: "POST",
            body: '{"after":1}',
            path: `/service/${PACKAGE}/replica/sync`,
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
                },
            },
            egress: "http://127.0.0.1:7470/.destack/egress",
        },
        identified: [instanceId, undefined],
        isRunning: false,
    });
});
