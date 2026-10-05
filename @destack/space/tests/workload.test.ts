import { expect, onTestFinished, test } from "@destack/test";
import { cp, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { notebook } from "@destack/notes";
import { notesService } from "@destack/notes/service";
import { notesDatabase } from "@destack/notes/stack";
import { sqliteProvider } from "@destack/db/sqlite";
import { Plan } from "@destack/resource";
import { identifier, type Identifier } from "@destack/schema";
import { createClient } from "@destack/service/client";
import { WorkloadReady, type WorkloadStart } from "@destack/service/workload";
import { CALLER_HEADER } from "@destack/service/authentication";
import { ServiceMount } from "@destack/service";
import { RequestId } from "@destack/service/request";
import {
    accessTables,
    accessRelationship,
    accessRole,
    anyone,
    principal,
    Relationship,
    Role,
} from "@destack/access";
import { copyScope } from "@destack/access/test";
import { account } from "@destack/account/object";
import type { DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Scope, Feed } from "@destack/sync";
import { Health } from "@destack/service/health";
import { implement, ServiceHandler } from "@destack/service/server";
import { spaceService } from "../src/service/index.ts";
import { space } from "../src/object/index.ts";
import { TABLE } from "@destack/db";
import { Package, type DeclarationDescription } from "@destack/package";
import type { Compilation } from "@destack/package/build";
import { outbox } from "@destack/service/outbox";
import { PackageBuilder } from "@destack/build";
import { linkDependencies, readDependencies } from "@destack/build/local";
import { spaceBuild } from "../src/build/index.ts";
import { pushThrough } from "../src/server/index.ts";
import { v7 } from "uuid";
import { relayAuthorizer } from "./fixture/space.ts";

/** The notes package: objects, a service serving them and one database, without workload code. */
const NOTES = fileURLToPath(new URL("../../notes/", import.meta.url));

/** The service package, whose instruments report request failures. */
const SERVICE_MANIFEST = new URL("../../service/package.json", import.meta.url);

/** The compiler settings of the copied package, standing alone outside the workspace. */
const TYPESCRIPT = {
    compilerOptions: {
        target: "ESNext",
        module: "Preserve",
        moduleResolution: "Bundler",
        lib: ["ESNext", "DOM", "DOM.Iterable"],
        types: [],
        strict: true,
        skipLibCheck: true,
        noEmit: true,
        allowImportingTsExtensions: true,
        verbatimModuleSyntax: true,
    },
    include: ["src/**/*.ts"],
};

/** The longest a server build of the notes package takes: about 10 s measured under load. */
const NOTES_BUILD_MILLISECONDS = 60_000;

test(
    "derive a workload serving a package's objects-only service and run it in its own process from a host's start message",
    { timeout: NOTES_BUILD_MILLISECONDS },
    async () => {
        // copy the notes package's objects, service and database beside its installed dependencies
        const directory = await mkdtemp(join(tmpdir(), "destack-notes-"));
        try {
            const source = join(directory, "source");
            await cp(NOTES, source, {
                recursive: true,
                filter: (path) =>
                    !["node_modules", "view", "tests", "vitest.config.ts"].includes(basename(path)),
            });
            const manifest = JSON.parse(await readFile(join(NOTES, "package.json"), "utf8"));
            const { "./view": _view, ...exports } = manifest.exports;
            await writeFile(
                join(source, "package.json"),
                JSON.stringify({
                    ...manifest,
                    exports,
                }),
            );
            await writeFile(join(source, "tsconfig.json"), JSON.stringify(TYPESCRIPT));
            await linkDependencies(NOTES, source);

            // build the server output
            await using builder = await PackageBuilder.start(source);
            await using build = await builder.build({
                dependencies: await readDependencies(NOTES),
                outputs: {
                    server: { kind: "module", runtime: "bun", bundle: true },
                },
            });

            // describe the derived workload like a declared one
            const output = build.manifest.outputs.server;
            const packageId = build.manifest.package.id;
            expect(output.workloads).toEqual({
                notes: {
                    entrypoint: "./workload/notes",
                    services: [{ packageId, name: "notes" }],
                    triggers: [],
                    resources: [{ packageId, name: "main" }],
                    secrets: [],
                    connections: [],
                    compute: { requests: {}, limits: {}, scaling: {} },
                },
            });

            // provision and migrate the notes database as a host's provider does
            const destination = join(directory, "build");
            await build.write(destination);
            const provider = sqliteProvider(pathToFileURL(`${directory}/resources/`));
            const record = {
                id: identifier("resource").parse("resource-01996ab0-0000-7000-8000-000000000001"),
                scope: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
                kind: "database" as const,
                spec: { tier: "zonal" as const },
                reference: null,
            };
            const { reference } = await provider.provision(record);
            const provisioned = { ...record, reference };
            const desired = [notesDatabase.state()];
            const plan = await provider.plan(provisioned, desired);
            await provider.apply(provisioned, desired, await Plan.digest(plan));

            // keep the space below its account in the cell's database, letting anyone reach it through a member role
            const cell = await TestDatabase.create("sqlite", notesDatabase, { isMigrated: true });
            onTestFinished(() => cell.close());
            const accountId = await seedSpace(cell.database, record.scope);

            // relay the cell's copies of the space's chain and end each snapshot
            const feed = new Feed(cell.database, accessTables);
            const resumed = Promise.withResolvers<void>();
            const relay = implement({ stream: spaceService.router.replica.stream }).$context<{
                request: Request;
            }>();
            const handler = new ServiceHandler(
                relay.router({
                    stream: relay.stream.handler(async function* ({ input, signal }) {
                        // note the runner resuming the account's copy after the space's copy arrives
                        if (input.after !== undefined && input.scope === accountId) {
                            resumed.resolve();
                        }

                        // stream the copy, ending a snapshot at its first complete page
                        const copy = relayAuthorizer.replicaOf(input);
                        yield* feed.subscribe(
                            copy.queries,
                            input.after,
                            signal!,
                            input.after === undefined ? { drain: AbortSignal.abort() } : {},
                        );
                    }),
                }),
                { service: spaceService, health: new Health("space"), authorize: async () => {} },
            );
            // receive the runner's telemetry as the cell's monitor does
            const exported: unknown[] = [];
            const holderServer = Bun.serve({
                hostname: "127.0.0.1",
                port: 0,
                fetch: async (request) => {
                    if (
                        new URL(request.url).pathname.startsWith(
                            "/.destack/egress/@destack/monitor/v1/",
                        )
                    ) {
                        exported.push(await request.json());

                        return Response.json({});
                    }

                    return (
                        (
                            await handler.handle(request, {
                                prefix: "/.destack/egress/@destack/space",
                                context: { request },
                            })
                        ).response ?? new Response(null, { status: 404 })
                    );
                },
            });
            onTestFinished(() => holderServer.stop(true));

            // start the runner with the host's first line and read the port it serves on
            const runner = Bun.spawn(
                ["bun", join(destination, output.exports["./workload/notes"]!)],
                {
                    stdin: "pipe",
                    stdout: "pipe",
                    stderr: "pipe",
                },
            );
            onTestFinished(() => runner.kill());
            const start: WorkloadStart = {
                journalKey: "00".repeat(32),
                instance: identifier("instance").parse(
                    "instance-01996ab0-0000-7000-8000-000000000003",
                ),
                scope: record.scope,
                installation: identifier("installation").parse(
                    "installation-01996ab0-0000-7000-8000-000000000009",
                ),
                bindings: {
                    main: {
                        resource: record.id,
                        kind: "database",
                        provider: "sqlite",
                        reference,
                    },
                },
                secret: "forwarding",
                egress: `${holderServer.url.origin}/.destack/egress`,
                sampling: 1,
            };
            void runner.stdin.write(`${JSON.stringify(start)}\n`);
            const line = (await runner.stdout.getReader().read()).value;
            const { port } = WorkloadReady.parse(JSON.parse(new TextDecoder().decode(line)));

            // wait until the runner has copied the space and resumes its account's copy
            await resumed.promise;

            // refuse a request without the host's secret, and serve a caller the host forwards
            const url = `http://127.0.0.1:${port}${ServiceMount.path(packageId)}`;
            const now = Date.now();
            const subject = principal.user.reference(
                "universe",
                "user-01996ab0-0000-7000-8000-000000000004",
            );
            const caller = {
                credential: { kind: "session", id: "session" },
                audience: packageId,
                scope: record.scope,
                subject,
                subjects: [subject],
                verifiedAt: now,
                expiresAt: now + 60_000,
            };
            const create = (headers: Record<string, string>) =>
                createClient(notesService, { url, headers }).notebook.create({
                    spaceId: record.scope,
                    requestId: RequestId.create(),
                    name: "Ideas",
                } as never);
            const refused = (headers: Record<string, string>) =>
                create(headers).then(
                    () => "created",
                    (error: { code: string; message: string }) => [error.code, error.message],
                );
            expect(await refused({})).toEqual(["UNAUTHORIZED", "invalid host secret"]);
            const created = await create({
                authorization: "Bearer forwarding",
                [CALLER_HEADER]: JSON.stringify(caller),
            });
            const { id, createdAt, updatedAt, ...fields } = created;
            expect([id.startsWith("notebook-"), createdAt === updatedAt, fields]).toEqual([
                true,
                true,
                {
                    name: "Ideas",
                    noteCount: 0,
                    owner: subject.id,
                    revision: 1,
                    scope: record.scope,
                    tags: {},
                },
            ]);

            // push a run's call as the installation through the host's ingress, which the notes policy refuses since users create notebooks, and replay that outcome under the run's one request
            const push = pushThrough({
                ingress: async (_installation, path, request, forwarded) => {
                    const headers = new Headers(request.headers);
                    headers.set("authorization", "Bearer forwarding");
                    forwarded.forward(headers);

                    return fetch(new Request(`${url}${path}`, new Request(request, { headers })));
                },
            });
            const installed = { id: start.installation, scope: record.scope, packageId };
            const mutation = {
                id: v7(),
                calls: [notebook.calls().create({ name: "Runs" })],
            };
            const refusal = {
                error: { code: "FORBIDDEN", message: "notebook is created by a user", status: 403 },
            };
            expect([
                await push(installed as never, mutation, AbortSignal.timeout(10_000)),
                await push(installed as never, mutation, AbortSignal.timeout(10_000)),
            ]).toEqual([refusal, refusal]);

            // fail a request whose forwarded caller is malformed
            expect(
                await refused({ authorization: "Bearer forwarding", [CALLER_HEADER]: "{" }),
            ).toEqual(["INTERNAL_SERVER_ERROR", "internal server error"]);

            // stop cleanly on the host's signal, exporting the telemetry still buffered
            runner.kill("SIGTERM");
            expect(await runner.exited).toBe(0);

            // export the failure to the monitor from the service's instruments in the package's resource
            const service = JSON.parse(await readFile(SERVICE_MANIFEST, "utf8"));
            expect(logRecord(exported, "service.request.failed")).toEqual({
                resource: {
                    attributes: [
                        {
                            key: "service.instance.id",
                            value: { stringValue: "instance-01996ab0-0000-7000-8000-000000000003" },
                        },
                        {
                            key: "service.name",
                            value: { stringValue: build.manifest.package.name },
                        },
                        {
                            key: "service.version",
                            value: { stringValue: build.manifest.package.version },
                        },
                    ],
                    droppedAttributesCount: 0,
                },
                scope: { name: service.name, version: service.version },
                record: {
                    timeUnixNano: expect.any(String),
                    observedTimeUnixNano: expect.any(String),
                    severityNumber: 17,
                    severityText: "ERROR",
                    body: {},
                    eventName: "service.request.failed",
                    attributes: [
                        { key: "exception.type", value: { stringValue: "SyntaxError" } },
                        {
                            key: "exception.message",
                            value: { stringValue: "JSON Parse error: Expected '}'" },
                        },
                        { key: "exception.stacktrace", value: { stringValue: expect.any(String) } },
                    ],
                    droppedAttributesCount: 0,
                    traceId: expect.stringMatching(/^[0-9a-f]{32}$/),
                    spanId: expect.stringMatching(/^[0-9a-f]{16}$/),
                    flags: 1,
                },
            });
        } finally {
            await rm(directory, { recursive: true, force: true });
        }
    },
);

test("refuse deriving a workload without one journal and one database keeping it and the audit outbox", () => {
    // declare an objects-only service, then a journal without the audit outbox
    const app = Package.parse({
        id: "package-01996ab0-0000-7000-8000-00000000000c",
        name: "@example/app",
        version: "2026.9.0",
    });
    const declare = (
        kind: string,
        name: string,
        description: DeclarationDescription["description"],
    ): DeclarationDescription => ({
        name,
        kind,
        package: app,
        constructor: { package: app, symbol: { module: "src/index.ts", name: "define" } },
        symbol: { package: app, symbol: { module: "src/index.ts", name } },
        source: { file: "src/index.ts", line: 0, column: 0 },
        description,
    });
    const service = declare("service", "notes", {
        name: "notes",
        protocol: "http",
        api: { name: "notes", procedures: [] },
        objects: ["note"],
        routes: [],
    });
    const journal = declare("journal", "journal", { name: "journal" });
    const table = (name: string) => ({
        package: app,
        table: { dialect: "sqlite", name, columns: [], constraints: [], indexes: [] },
    });
    const database = declare("resource", "main", {
        name: "main",
        kind: "database",
        spec: { tier: "zonal" },
        tables: { sqlite: [table("example__app__journal")], postgresql: [] },
    });

    // compile a Bun server output of the declarations, listing what is missing
    const refusal = (declarations: DeclarationDescription[]) => {
        const compilation: Compilation = {
            package: app,
            directory: "/package",
            runtime: "bun",
            declarations,
            modules: [],
            locate: (declaration) => ({ file: "/package/src/index.ts", export: declaration.name }),
            entry: () => {},
        };
        try {
            void spaceBuild.compile!(compilation);
        } catch (error) {
            return (error as Error).message;
        }
    };
    expect([refusal([service]), refusal([service, journal, database])]).toEqual([
        "objects-only services need one journal, found none",
        `objects-only services need one database keeping example__app__journal and ${outbox[TABLE].sqlName}, found none`,
    ]);
});

/** Find the log record of an event among OTLP/JSON log exports, with its resource and scope. */
function logRecord(exports: readonly unknown[], event: string): unknown {
    // search every resource and scope of every export
    for (const request of exports as {
        resourceLogs?: {
            resource: unknown;
            scopeLogs: { scope: unknown; logRecords: { eventName?: string }[] }[];
        }[];
    }[]) {
        for (const group of request.resourceLogs ?? []) {
            for (const { scope, logRecords } of group.scopeLogs) {
                const record = logRecords.find((each) => each.eventName === event);
                if (record !== undefined) {
                    return { resource: group.resource, scope, record };
                }
            }
        }
    }

    throw new Error(`no exported log record of event ${event}`);
}

/** Record a space below its account and let anyone read it through a member role, returning the account. */
async function seedSpace(
    database: DatabaseConnection,
    spaceId: Identifier<"space">,
): Promise<Identifier<"account">> {
    // record the account and the space
    const accountId = identifier("account").parse("account-01996ab0-0000-7000-8000-000000000006");
    await copyScope(database, account.reference(Scope.universe.id, accountId));
    await copyScope(database, space.reference(accountId, spaceId));

    // grant the space's read permission to a member role
    const role = identifier("role").parse("role-01996ab0-0000-7000-8000-000000000007");
    await database.insert(accessRole).values({
        id: role,
        createdAt: 0,
        updatedAt: 0,
        scope: spaceId,
        name: "member",
        description: "Reads the space",
    });
    const read = space.permission("read");
    await Role.permit(database, role, spaceId, [
        { packageId: read.packageId, type: read.type, name: read.name },
    ]);

    // relate anyone to the space through the role
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: identifier("relationship").parse(
                    "relationship-01996ab0-0000-7000-8000-000000000008",
                ),
                object: space.reference(accountId, spaceId),
                role,
                subject: anyone.reference("*", "*"),
                createdAt: 0,
                expiresAt: null,
            },
            spaceId,
        ),
    );

    return accountId;
}
