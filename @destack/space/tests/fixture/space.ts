import type { InstanceSpec, Runtime } from "@destack/host/runtime";
import { reconciliation, testJournalKey } from "@destack/service/test";
import type { ServerRuntime } from "@destack/package/runtime";
import { onTestFinished } from "@destack/test";
import { setting } from "@destack/setting/object";
import { Authorization, Authorizer, principal, type Subject } from "@destack/access";
import { copyScope } from "@destack/access/test";
import { account, organisation, user } from "@destack/account/object";
import { DirectoryDatabase } from "@destack/directory";
import { directoryTables } from "@destack/directory";
import { ObjectServer } from "@destack/object/server";
import { AuditRecorder } from "@destack/audit";
import { type DatabaseConnection, type Dialect, eq, type Table } from "@destack/db";
import type { Change } from "@destack/db/log";
import { defineDatabase } from "@destack/db/declare";
import { TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { until } from "@destack/service/timer";
import { canonicalize } from "@destack/schema/json";
import { connect } from "../../src/client/index.ts";
import * as object from "../../src/object/index.ts";
import {
    implementService,
    type OpenBuild,
    type GlobalTier,
    type SpaceServiceOptions,
} from "../../src/server/index.ts";
import { type PackageManifest, BuildReader } from "@destack/package/manifest";
import { describeFile } from "@destack/package/file";
import type { Upgrade } from "@destack/resource";
import { spaceTables } from "../../src/stack/index.ts";

/** The authorizer a test relay serves copies with, knowing the inherited setting rows and the accounts' own rows. */
export const relayAuthorizer = new Authorizer(
    [setting.policy, account.policy, user.policy, organisation.policy],
    [setting.mapping, account.mapping, user.mapping],
);

/** The identities the scenarios name. */
export const ids = {
    account: identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    region: identifier("region").parse("region-01996ab0-0000-7000-8000-000000000003"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: identifier("installation").parse("installation-01996ab0-0000-7000-8000-000000000005"),
    run: identifier("run").parse("run-01996ab0-0000-7000-8000-000000000006"),
    host: identifier("host").parse("host-01996ab0-0000-7000-8000-000000000007"),
};

/** Declare a cell's regional database: the space's tables beside other packages'. */
export function cellDatabase(tables: readonly Table[] = []) {
    return defineDatabase({ name: "main", tier: "regional", tables: [...spaceTables, ...tables] });
}

/** Keep the account's scope and its own row, as its home and the cells copying its chain do. */
export async function copyAccount(database: DatabaseConnection): Promise<void> {
    const now = Date.now();
    await copyScope(database, account.reference("universe", ids.account));
    await database.insert(account.table).values({
        id: ids.account,
        scope: "universe" as typeof account.table.$inferInsert.scope,
        handle: "acme",
        name: "Acme",
        kind: "shared",
        defaultResidency: "eu",
        createdAt: now,
        updatedAt: now,
    });
}

/** Keep a space its owner owns, in a database closed after the test, with other packages' tables beside it. */
export async function openSpace(
    tables: readonly Table[] = [],
    dialect: Dialect = "sqlite",
): Promise<DatabaseConnection> {
    // keep the account's scope and the space within it, beside other packages' tables
    const test = await TestDatabase.create(dialect, cellDatabase(tables), {
        isMigrated: true,
    });
    onTestFinished(() => test.close());
    const database = test.database;
    const now = Date.now();
    await copyAccount(database);
    await database.insert(object.space.table).values({
        id: ids.space,
        scope: ids.account,
        name: "personal",
        createdAt: now,
        updatedAt: now,
    });

    // make the owner the space's owner
    const owner = principal.user.reference("universe", "owner");
    await new Authorization(
        new Authorizer([object.space.policy], [object.space.mapping]),
        database,
        () => ({ subjects: [owner], now, attributes: {} }),
    ).create(object.space.reference(ids.account, ids.space), { owner });

    return database;
}

/** Record nothing of the calls the fixtures make. */
export const audit = AuditRecorder.service<DatabaseConnection>(
    { append: async () => {} },
    {
        package: { id: ids.package, name: "@destack/space", version: "2026.9.0" },
        service: "space",
    },
);

/** Reconcile one key of a served object type's controller once, as the control loop does. */
export async function reconcile(
    server: Pick<ObjectServer, "controllers" | "database">,
    name: string,
    key: Readonly<Record<string, unknown>>,
): Promise<number | undefined> {
    // find the controller
    const controller = server.controllers().find((each) => each.name === name);
    if (controller === undefined) {
        throw new TypeError(`the fixture serves no controller ${name}`);
    }

    // see the key changed at each commit after the reconciliation waits, as the loop following the log would
    const { log } = server.database;
    const settled = new AbortController();
    const changed = async () => {
        const { sequence } = await log.position();
        await log.until(async () => (await log.position()).sequence > sequence, settled.signal);
    };
    const signal = new AbortController().signal;
    try {
        return await controller.reconcile(canonicalize(key), { signal, changed });
    } finally {
        settled.abort();
    }
}

/** Reconcile every resource of a space, as the control loop does once the space changes. */
export async function reconcileResources(
    server: Pick<ObjectServer, "controllers" | "database">,
    spaceId: string,
): Promise<void> {
    const rows = await server.database
        .select({ id: object.resource.table.id })
        .from(object.resource.table)
        .where(eq(object.resource.table.scope, identifier("space").parse(spaceId)));
    for (const row of rows) {
        await reconcile(server, "resource", { id: row.id });
    }
}

/** Reconcile the keys a change selects for a served object type's controller, as the control loop does. */
export async function wake(
    server: Pick<ObjectServer, "controllers">,
    name: string,
    change: Change,
): Promise<void> {
    const controller = server.controllers().find((each) => each.name === name)!;
    for (const key of (await controller.keys?.(change)) ?? []) {
        await controller.reconcile(key, reconciliation(new AbortController().signal));
    }
}

/** Fill a space service's options with the fixture's region, builds and runtime, following nothing. */
export async function spaceOptions(
    database: DatabaseConnection,
    options: Partial<SpaceServiceOptions> = {},
): Promise<SpaceServiceOptions> {
    return {
        database,
        journalKey: testJournalKey,
        global: options.global ?? (await openGlobal()),
        cell: { regionId: ids.region },
        providers: [],
        declared: [],
        openBuild,
        bindables: [],
        runtimes: [new TestRuntime()],
        runs: {
            push: async (target) => {
                throw new TypeError(`the fixture pushes no calls to ${target.id}`);
            },
            verify: async () => {
                throw new TypeError("the fixture verifies no lent authority");
            },
        },
        audit,
        ...options,
    };
}

/** The workload every fixture build's server output has. */
export const workload = {
    entrypoint: ".",
    services: [],
    triggers: [],
    resources: [],
    secrets: [],
    connections: [],
    compute: {},
};

/** The upgrades of the fixture's later releases, by release, the last naming itself as its predecessor. */
export const UPGRADES: Readonly<Record<string, Upgrade>> = {
    "2026.12.0": { from: "2026.12.0", steps: [] },
    "2026.10.0": {
        from: "2026.9.0",
        steps: [
            {
                action: "create",
                target: "service/notes/procedure/count",
                risk: "safe",
                detail: "add procedure count",
            },
        ],
    },
    "2026.11.0": {
        from: "2026.10.0",
        steps: [
            {
                action: "delete",
                target: "object/note/relation/editor",
                risk: "backward-incompatible",
                detail: "remove: data stored under it no longer applies",
            },
        ],
    },
};

/** Open every build at its release with a home view, a main workload, no settings, and the release's upgrade. */
export const openBuild: OpenBuild = async (packageId, build) => {
    // describe the release's upgrade file
    const version = build.kind === "release" ? build.version : "2026.9.0";
    const upgrade = UPGRADES[version];
    const bytes = new TextEncoder().encode(JSON.stringify(upgrade ?? null));
    const file = await describeFile("manifest/upgrade.json", "application/json", bytes);

    return new BuildReader(
        {
            package: { id: packageId, name: "@test/package", version },
            descriptions: {},
            ...(upgrade === undefined ? {} : { upgrade: { package: resourcePackage, file } }),
            outputs: {
                browser: {
                    runtime: "browser",
                    emit: true,
                    workloads: {},
                    views: { home: { entrypoint: "view.js", permissions: [] } },
                },
                server: {
                    runtime: "bun",
                    emit: true,
                    workloads: { main: workload },
                    views: {},
                },
            },
        } as unknown as PackageManifest,
        async (path) => {
            if (upgrade !== undefined && path === file.path) {
                return bytes;
            }
            throw new TypeError(`the fixture build of ${packageId} has no ${path}`);
        },
    );
};

/** The package defining the upgrade file format. */
const resourcePackage = {
    id: "package-01a0c80b-614f-73eb-9845-7906f2b7cfe0",
    name: "@destack/resource",
    version: "2026.9.0",
};

/** A runtime recording the instances it starts and stops, as a host would run them. */
export class TestRuntime implements Runtime {
    /** The server runtime. */
    readonly name: ServerRuntime;
    /** The runs started, in order. */
    readonly started: InstanceSpec[] = [];
    /** The instances stopped, in order. */
    readonly stopped: string[] = [];

    /** Run instances as a server runtime, bun by default. */
    constructor(name: ServerRuntime = "bun") {
        this.name = name;
    }

    /** Record a run started. */
    async start(run: InstanceSpec): Promise<void> {
        this.started.push(run);
    }

    /** Record an instance stopped. */
    async stop(instanceId: string): Promise<void> {
        this.stopped.push(instanceId);
    }

    /** Report an instance running once started and until stopped. */
    isRunning(instanceId: string): boolean {
        return (
            this.started.some((run) => run.instanceId === instanceId) &&
            !this.stopped.includes(instanceId)
        );
    }

    /** Identify no instance, since no runner has a secret. */
    identify(): undefined {
        return undefined;
    }

    /** Refuse requests, since no instance serves. */
    async fetch(instanceId: string): Promise<Response> {
        throw new TypeError(`the test runtime serves no requests of ${instanceId}`);
    }

    /** Receive no webhook requests. */
    async receive(instanceId: string): Promise<Response> {
        throw new TypeError(`the test runtime receives no webhooks of ${instanceId}`);
    }
}

/** Copy nothing of the universe and reach no peer's zone. */
const idle: Pick<GlobalTier, "replicas" | "relay"> = {
    replicas: { stream: (_request, signal) => stream(signal) },
    relay: async (cell) => {
        throw new TypeError(`the fixture reaches no zone relay of ${cell}`);
    },
};

/** Yield nothing until a signal aborts. */
async function* stream(signal: AbortSignal): AsyncGenerator<never> {
    await until(signal);
    yield* [];
}

/** Open a global tier of its own, closed after the test: a directory and key index in one database, following nothing. */
export async function openGlobal(
    tier: Partial<Pick<GlobalTier, "replicas" | "relay">> = {},
): Promise<GlobalTier> {
    const global = await TestDatabase.create("sqlite", directoryTables, {
        isMigrated: true,
    });
    onTestFinished(() => global.close());

    return { ...idle, ...tier, directory: new DirectoryDatabase(global.database) };
}

/** Serve a space's database to each request's user or host and return a client per user or host. */
export async function serveSpace(
    database: DatabaseConnection,
    global?: GlobalTier,
    options: Partial<SpaceServiceOptions> = {},
): Promise<
    (
        user: string,
        options?: { readonly isAuthenticatedNow?: boolean },
    ) => ReturnType<typeof connect>
> {
    // authenticate each request as the user its header names
    const server = Server.start({
        ...implementService(
            await spaceOptions(database, {
                ...options,
                ...(global === undefined ? {} : { global }),
            }),
        ),
        audience: ids.package,
        resources: new ResourceContext(),
        health: new Health("space"),
        authenticate: async (request) => {
            // take the host or the installation a header names, or else the user
            const host = request.headers.get("x-host");
            const installation = request.headers.get("x-installation");
            const subject =
                host !== null
                    ? principal.host.reference(ids.account, host)
                    : installation !== null
                      ? principal.installation.reference(ids.space, installation)
                      : principal.user.reference("universe", request.headers.get("x-user")!);

            return subjectCaller(subject, request.headers.has("x-authenticated-now"));
        },
        authorizeHost: async () => {},
        drainTimeout: 1000,
    });
    onTestFinished(() => server.close());

    return (caller, options = {}) =>
        connect({
            url: "http://space.test",
            headers: {
                ...(caller.startsWith("host-")
                    ? { "x-host": caller }
                    : caller.startsWith("installation-")
                      ? { "x-installation": caller }
                      : { "x-user": caller }),
                ...(options.isAuthenticatedNow === true ? { "x-authenticated-now": "1" } : {}),
            },
            fetch: (request) => server.fetch(request),
        });
}

/** Build a caller acting as a subject, as the fixture's authentication does, at the highest assurance when it authenticated just now. */
export function subjectCaller(subject: Subject, isAuthenticatedNow = false): Caller {
    const now = Date.now();

    return new Caller({
        credential: { kind: "fixture", id: "fixture-1" },
        audience: ids.package,
        subject,
        subjects: [subject],
        ...(isAuthenticatedNow ? { assurance: { level: 2, authenticatedAt: now } } : {}),
        verifiedAt: now,
        expiresAt: now + 60_000,
    });
}
