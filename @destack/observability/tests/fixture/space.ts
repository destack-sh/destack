import { SettingValue, setting } from "@destack/setting/object";
import type { ObjectType } from "@destack/object";
import type { Call } from "@destack/sync";
import { STACKS } from "@destack/account/test";
import { AccessContext, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { account, space, user } from "@destack/account/object";
import { type DatabaseConnection, defineDatabase, type Table } from "@destack/db";
import * as sqlite from "@destack/db/bun";
import { TestDatabase, reconcileOnce } from "@destack/db/test";
import type { EventStore } from "@destack/event";
import { LocaleTag } from "@destack/locale";
import { notification, type NotificationType } from "@destack/notification";
import notificationServer from "@destack/notification/server";
import { NotificationFixture } from "@destack/notification/test";
import { notificationTables } from "@destack/notification/stack";
import { call, journal } from "@destack/object";
import { EphemeralStorage, ObjectStore } from "@destack/object/server";
import { testEvents } from "@destack/object/test";
import { Package, PackageId } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";
import type { Controller } from "@destack/db";
import { RequestId, Scope, type Subject } from "@destack/sync";
import { subjectContext, testCallKey, testServerContext } from "@destack/object/test";
import { present, schema } from "@destack/schema";
import { mention, reply, thread } from "@destack/social";
import socialPackage from "@destack/social/package";
import socialServer from "@destack/social/server";
import { socialTables } from "@destack/social/stack";
import { build, installation, installationRevision } from "@destack/stack/object";
import { buildIndexServers } from "@destack/stack/test";
import { onTestFinished } from "@destack/test";
import {
    alert,
    alertRule,
    firingAlert,
    issue,
    regressedIssue,
    resolvedAlert,
    resolvedIssue,
    RESPOND_CAPABILITY,
    silence,
} from "../../src/object/index.ts";
import observabilityPackage from "../../src/package.ts";
import observabilityServer from "../../src/server.ts";
import { VirtualClock } from "@destack/environment/simulation";

/** The time every scenario starts at: 2027-01-15, mid-month, in Unix milliseconds. */
export const START = Date.UTC(2027, 0, 15, 12);

/** The identities the scenarios name. */
export const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
    instance: schema.identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000006"),
    machine: schema.identifier("machine").parse("machine-01996ab0-0000-7000-8000-000000000007"),
    owner: schema.identifier("user").parse("user-01996ab0-0000-7000-8000-000000000008"),
};

/** The notes package, as its builds name it. */
export const notes = { id: ids.package, name: "@example/notes", version: "2026.9.0" };

/** The principals the scenarios act as: alice reads the space, the account's machine serves it. */
export const actors = {
    alice: principal.user.reference(Scope.universe.id, "user-01996ab0-0000-7000-8000-000000000009"),
    machine: principal.machine.reference(ids.account, ids.machine),
};

/** The builds of the packages declaring the space's notifications: observability and social. */
const DECLARING = await Promise.all([
    declaring(regressedIssue.package, [regressedIssue, resolvedIssue, firingAlert, resolvedAlert]),
    declaring(mention.package, [mention, thread, reply]),
]);

/** How a scenario serves its space. */
export interface SpaceOptions {
    /** The clock the store runs at, a new one at `START` by default. */
    readonly clock?: VirtualClock;
}

/** A space with observability over a bare store, alice reading it and notes installed. */
export interface ObservedSpace {
    /** The space's store. */
    readonly store: ObjectStore;
    /** The space's event log. */
    readonly events: EventStore;
    /** The clock the store runs at, starting at `START`. */
    readonly clock: VirtualClock;
    /** The failures observability reported. */
    readonly failures: unknown[];
    /** The controllers the space's packages run: the store's own and observability's. */
    readonly controllers: readonly Controller[];
    /** Answer a request at observability's mount, such as an OTLP export. */
    readonly fetch: (request: Request, caller: Subject) => Promise<Response | undefined>;
}

/** The types observability declares. */
const OBSERVABILITY_OBJECTS: readonly ObjectType[] = Object.values(observabilityPackage.objects);

/** Serve observability in a new space of a SQLite database, closed after the test. */
export async function openSpace(options: SpaceOptions = {}): Promise<ObservedSpace> {
    // migrate the space's tables
    const clock = options.clock ?? new VirtualClock(START);
    const tables: readonly Table[] = [
        journal,
        ...call.tables,
        ...[...OBSERVABILITY_OBJECTS, installation, installationRevision, build, user].flatMap(
            (object) => object.tables,
        ),
        account.table,
        space.table,
        ...setting.tables,
        ...notificationTables,
        ...socialTables,
    ];
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [...new Set(tables)] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;

    // serve observability as its installation in the space, responding to alerts
    const observability = {
        ...(await testServerContext(Package.of(observabilityPackage), ids.space)),
        capabilities: [RESPOND_CAPABILITY],
    };

    // keep the space's events and serve observability, notifications and comments as the space
    const events = await testEvents(
        database,
        OBSERVABILITY_OBJECTS.filter((object) => object.storage === "events").map((object) =>
            object.eventKind(),
        ),
        clock,
        SettingValue.retention(database),
    );
    const failures: unknown[] = [];

    // keep social's presence in memory, beside the space's own database
    const ephemeral = present(
        await EphemeralStorage.open(
            Object.values(socialPackage.objects),
            EphemeralStorage.channel(database),
            (kept) => sqlite.connectBunSqlite(":memory:", kept),
            { clock, report: (error) => failures.push(error) },
        ),
        "the social package's ephemeral storage",
    );
    onTestFinished(() => ephemeral[Symbol.asyncDispose]());
    const store = new ObjectStore({
        // serve the space's packages, and the index of the builds declaring notifications
        servers: [
            observabilityServer.at(observability),
            notificationServer,
            socialServer,
            ...(await buildIndexServers(database, DECLARING)),
        ],
        chain: async () => undefined,
        clock,
        database,
        events,
        ephemeral,
        callKey: testCallKey,
        origin: { package: Package.of(observabilityPackage) },
        principal: principal.space.reference(Scope.universe.id, ids.space),
    });
    const packages = store.packages;

    // record the account and the space, the space owning itself and alice reading it
    const reference = space.reference(ids.account, ids.space);
    const access = await recordSpace(database);
    await access.bindOwner(reference, store.principal, 0);
    await access.replicateRole(
        reference,
        {
            name: "observer",
            description: "Reads and triages the space's issues and alerts",
            permissions: [
                issue.permission("read"),
                issue.permission("write"),
                alertRule.permission("read"),
                alertRule.permission("write"),
                alertRule.permission("manage"),
                alert.permission("read"),
                alert.permission("write"),
                silence.permission("read"),
                silence.permission("write"),
            ],
        },
        actors.alice,
    );

    // grant observability's installation what its package requests in the space
    await access.replicateRole(
        reference,
        {
            name: "installation/observability",
            description: "What observability requests in this space",
            permissions: [...(observabilityPackage.permissions.installation?.space ?? [])],
        },
        principal.installation.reference(ids.space, observability.installation),
    );

    // install notes
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        name: "notes",
        selection: { kind: "release", version: notes.version },
        createdAt: START,
        updatedAt: START,
    });

    // install the packages declaring notifications, and keep alice's profile
    await NotificationFixture.install(database, ids.space, DECLARING, START);
    await NotificationFixture.person(
        database,
        actors.alice,
        { name: "Alice", locale: LocaleTag.parse("en"), timeZone: "UTC" },
        START,
    );

    // answer requests at observability's mount as a caller
    const served = present(packages[0], "observability's implementation");
    const fetch = async (request: Request, caller: Subject) => {
        if (served.fetch === undefined) {
            throw new TypeError("observability answers no adapters");
        }

        return served.fetch(request, subjectContext(caller, ids.space, { clock }));
    };

    const controllers = packages.flatMap((each) => each.controllers ?? []);

    return { store, events, clock, failures, fetch, controllers };
}

/** Record the account paying for itself and its space, as the space's replicas keep them. */
export async function recordSpace(database: DatabaseConnection): Promise<AccessFixture> {
    // record the account under its owner and the space under the account
    const record = { createdAt: START, updatedAt: START };
    await database.insert(account.table).values({
        ...record,
        id: ids.account,
        scope: ids.owner,
        handle: "acme",
        name: "Acme",
        kind: "personal",
        residencyId: "eu",
    });
    await database.insert(space.table).values({
        ...record,
        id: ids.space,
        scope: ids.account,
        name: "notes",
        stack: STACKS.space,
    });

    // record their scopes, the space its own partition
    const access = new AccessFixture(database);
    await access.replicateScope(user.reference(Scope.universe.id, ids.owner));
    await access.replicateScope(account.reference(ids.owner, ids.account));
    await access.replicateScope(space.reference(ids.account, ids.space), "own");

    return access;
}

/** Run calls as one mutation of a caller at the store's clock. */
export async function mutate<const Calls extends readonly Call[]>(
    store: ObjectStore,
    caller: Subject,
    calls: Calls,
) {
    const now = store.clock.now();

    return store.mutate(AccessContext.of(Caller.of(caller), Date.now()), {
        id: RequestId.create(now),
        calls,
    });
}

/** Run a controller of the store over every key it lists, as its loop does once. */
export async function reconcile(observed: ObservedSpace, name: string): Promise<void> {
    const controller = present(
        observed.controllers.find((each) => each.name === name),
        `the space's ${name} controller`,
    );
    await reconcileOnce(controller, observed.store.database);
}

/** Write a package's build declaring its notifications, as its release describes them. */
function declaring(owner: Package, declared: readonly NotificationType[]) {
    return MemoryBuild.of(
        owner,
        declared.map((each) => ({
            kind: "notification",
            kindPackageId: notification.package.id,
            name: each.name,
            description: each.describe(),
        })),
    );
}
