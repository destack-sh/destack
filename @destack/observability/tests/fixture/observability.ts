import { space } from "@destack/account/object";
import { onTestFinished } from "@destack/test";
import { accessRole, Authorization, Authorizer, principal } from "@destack/access";
import { and, type DatabaseConnection, defineDatabase, eq } from "@destack/db";
import { type Event, EventStore, DatabasePersonalKeyring, type Route } from "@destack/event";
import { eventTables } from "@destack/event/stack";
import type { Sku } from "@destack/finance";
import { usage } from "@destack/finance/declare";
import { LocalKeyring } from "@destack/identity";
import { notificationTables } from "@destack/notification/stack";
import { PackageId } from "@destack/package";
import { MemoryBucket } from "@destack/bucket/test";
import { ResourceContext } from "@destack/resource/context";
import { present, schema } from "@destack/schema";
import type { Authentication } from "@destack/service/authentication";
import { TestBearer } from "@destack/service/test";
import { createClient } from "@destack/service/client";
import { implementEvents } from "@destack/event/server";
import { eventService } from "@destack/event/service";
import { type Controller, ControlLoop } from "@destack/service/control";
import { isServiceError } from "@destack/service/error";
import { Health } from "@destack/service/health";
import { Outbox, outbox } from "@destack/service/outbox";
import { Server, type ServiceImplementation } from "@destack/service/server";
import { socialTables } from "@destack/social/stack";
import { installation } from "@destack/space/object";
import type { OpenBuild } from "@destack/space/server";
import { serveSpace } from "@destack/space/server";
import { spaceCopies } from "@destack/space/stack";
import { openBuild, SpaceFixture, spaceDatabase } from "@destack/space/test";
import type { OtlpReceiver } from "@destack/telemetry/otlp";
import { OBSERVABILITY_KINDS } from "../../src/event/index.ts";
import { implementObservability, type Reading, TelemetryPolicy } from "../../src/server/index.ts";
import { observabilityService } from "../../src/service/index.ts";
import { observabilityTables } from "../../src/stack/index.ts";

/** The space server's controllers of observability's objects: the alert rules' evaluation. */
const OBJECT_CONTROLLERS: ReadonlySet<string> = new Set(["alert-rule"]);

/** The event kinds a test space keeps beside its calls: telemetry, analytics and usage. */
const KINDS = [...OBSERVABILITY_KINDS, usage];

/** A space's database keeping its events, issues, alert rules, alerts and the visitor salt beside the space's tables, with the comments, subscriptions and notifications on them. */
export const observedDatabase = defineDatabase({
    name: "space",
    tables: [
        ...new Set([
            ...spaceDatabase.tables.filter((table) => !spaceDatabase.copies(table)),
            ...eventTables(KINDS),
            ...observabilityTables,
            ...notificationTables,
            ...socialTables,
            outbox,
        ]),
    ],
    copies: spaceCopies,
});

/** Run some controllers over a database until the test ends, keeping each failure but a conflict, which the loop retries. */
function run(
    database: DatabaseConnection,
    controllers: readonly Controller[],
    failures: unknown[],
): void {
    const stopping = new AbortController();
    const running = new ControlLoop(database, controllers, {
        report: (_controller, _key, error) => {
            if (!isServiceError(error) || error.code !== "CONFLICT") {
                failures.push(error);
            }
        },
    }).run(stopping.signal);
    onTestFinished(async () => {
        stopping.abort();
        await running;
    });
}

/** The identities the scenario names. */
export const ids = {
    account: schema.identifier("account").parse("account-01996ab0-0000-7000-8000-000000000001"),
    space: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002"),
    package: PackageId.parse("package-01996ab0-0000-7000-8000-000000000004"),
    notes: schema
        .identifier("installation")
        .parse("installation-01996ab0-0000-7000-8000-000000000005"),
    region: schema.identifier("region").parse("region-01996ab0-0000-7000-8000-000000000003"),
    instance: schema.identifier("instance").parse("instance-01996ab0-0000-7000-8000-000000000006"),
};

/** The notes package, as its workload names itself. */
export const notes = { id: ids.package, name: "@example/notes", version: "2026.9.0" };

/** How a test serves observability: the builds it opens, what it measures and where routed copies go. */
export interface ObservabilityTestOptions {
    /** Open a package's build in the space, the space fixture's by default. */
    readonly openBuild?: OpenBuild;
    /** Read what the space uses, nothing by default. */
    readonly measure?: (from: number, to: number) => Promise<readonly Reading[]>;
    /** The SKUs rating each meter, none by default, which appends no usage. */
    readonly skus?: readonly Sku[];
    /** Name the scopes a route copies a scope's events to, none by default. */
    readonly targets?: (route: Route, scope: string) => Promise<readonly string[]>;
    /** Deliver routed copies, nowhere by default. */
    readonly deliver?: (kind: string, events: readonly Event[]) => Promise<void>;
}

/** Serve a space's observability over its space's events, where alice owns the space and notes is installed, authenticating by bearer name. */
export async function serveObservability(options: ObservabilityTestOptions = {}) {
    // keep alice's space with the notes installation
    const fixture = await SpaceFixture.open({ database: observedDatabase });
    const database = fixture.database;
    await createSpace(database);

    // keep the space's events over its database, a memory bucket and a generated keyring, on a clock the test sets
    const clock = { now: Date.now() };
    const failures: unknown[] = [];
    const bucket = new MemoryBucket(() => clock.now);
    const keyring = await LocalKeyring.read(LocalKeyring.generate());
    const events = new EventStore({
        database,
        kinds: KINDS,
        files: () => bucket,
        personal: new DatabasePersonalKeyring(database, keyring),
        outbox: new Outbox(database),
        targets: options.targets ?? (async () => []),
        deliver: async (kind, copies) => options.deliver?.(kind, copies),
        policy: TelemetryPolicy.of(database),
        now: () => clock.now,
        report: (error) => failures.push(error),
    });

    // serve the observability kind beside the space's objects
    const implementation = implementObservability({
        events,
        spaces: {
            openBuild: options.openBuild ?? openBuild,
            measure: options.measure ?? (async () => []),
            ...(options.skus === undefined ? {} : { skus: options.skus }),
        },
        report: (error) => failures.push(error),
    });
    const served = serveSpace({
        ...(await fixture.options({ extensions: [] })),
        extensions: [implementation],
    });
    const { server: objects, services } = served;
    const observed = implementation.receiver(objects);
    const server = listen(present(services[1], "the observability service"));

    // serve the space's events to readers as the event service
    const eventServer = listen(
        implementEvents({ store: events, access: objects.access, unmasked: objects.unmasked }),
    );

    // run the space server's controllers of observability's objects, which serve beside its first service
    const controlled = present(services[0], "the space service").controllers ?? [];
    run(
        database,
        controlled.filter((controller) => OBJECT_CONTROLLERS.has(controller.name)),
        failures,
    );

    // settle the store's flushes, compactions and expiries at the clock's time
    const settle = async () => {
        const signal = new AbortController().signal;
        for (const controller of events.controllers) {
            for (const key of await controller.list()) {
                await controller.reconcile(key, {
                    signal,
                    changed: () => new Promise<void>(() => {}),
                });
            }
        }
    };

    return {
        server,
        eventServer,
        events,
        receive: (...exported: Parameters<OtlpReceiver["receive"]>) =>
            observed.receive(...exported),
        objects,
        database,
        failures,
        clock,
        settle,
    };
}

/** Serve an observability implementation over HTTP to callers authenticating by bearer name, closed as the test finishes. */
function listen(implementation: ServiceImplementation): Server {
    // serve at once, as the fixture writes the copies itself and no snapshot ever arrives
    const { ready: _ready, ...served } = implementation;
    const server = Server.start({
        ...served,
        audience: observabilityService.package.id,
        drainTimeout: 1000,
        health: new Health("observability"),
        resources: new ResourceContext(),
        authorizeMachine: async () => {},
        authenticate: async (request) => authenticate(request),
    });
    onTestFinished(() => server.close());

    return server;
}

/** Make alice an owner of the fixture's space and install notes in it. */
export async function createSpace(database: DatabaseConnection): Promise<void> {
    // bind the space's owner role to alice as the fixture's owner
    const now = Date.now();
    const [role] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(and(eq(accessRole.scope, ids.space), eq(accessRole.name, "owner")));
    await new Authorization(new Authorizer([space.policy], [space.mapping]), database, () => ({
        subjects: [principal.user.reference("universe", "owner")],
        now,
        attributes: {},
    })).grant({
        object: space.reference(ids.account, ids.space),
        role: present(role, "the space's owner role").id,
        subject: principal.user.reference("universe", "alice"),
    });

    // install notes
    await database.insert(installation.table).values({
        id: ids.notes,
        scope: ids.space,
        packageId: ids.package,
        role: "application",
        alias: "notes",
        selection: { kind: "release", version: "2026.9.0" },
        createdAt: now,
        updatedAt: now,
    });
}

/** Authenticate the installation and the users by the bearer name. */
export function authenticate(request: Request): Authentication {
    // authenticate notes as its installation and other names as users
    const name = TestBearer.read(request);
    const subject =
        name === "notes"
            ? principal.installation.reference(ids.space, ids.notes)
            : principal.user.reference("universe", name);

    return TestBearer.authenticate(subject, observabilityService.package.id, Date.now());
}

/** Open an event service client acting as a named caller. */
export function eventClient(server: Server, name: string) {
    return createClient(eventService, {
        url: "https://observability.test",
        headers: { authorization: `Bearer ${name}` },
        fetch: (request: Request) => server.fetch(request),
    });
}

/** Open an observability client acting as a named caller. */
export function client(server: Server, name: string) {
    return createClient(observabilityService, {
        url: "https://observability.test",
        headers: { authorization: `Bearer ${name}` },
        fetch: (request: Request) => server.fetch(request),
    });
}
