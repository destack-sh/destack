import { allowanceShape, universeAllowanceShape } from "@destack/account/object";
import { accessRelationship, principal, Relationship } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { Account, account, organisation, user } from "@destack/account/object";
import type { Dialect } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { LATEST_TAG } from "@destack/forge/object";
import { ObjectServer } from "@destack/object/server";
import type { Package, PackageId } from "@destack/package";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { found, type Identifier, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { reconciliation, testCallKey } from "@destack/service/test";
import { Scope, Subject } from "@destack/sync";
import { v7 } from "uuid";
import { connect } from "../client/index.ts";
import { EventStore } from "@destack/event";
import { MemoryBucket } from "@destack/bucket/test";
import { usage } from "../meter/usage.ts";
import { EntitlementController } from "../server/entitlement.ts";
import { MeterUsage, UsageController } from "../server/usage.ts";
import { admitUsage } from "../server/server.ts";
import { eventService } from "@destack/event/service";
import { implementEvents } from "@destack/event/server";
import { createClient } from "@destack/service/client";
import { Feature } from "../feature/index.ts";
import { describeFeature, describeMeter, describeSku } from "../inspect/index.ts";
import type { Meter } from "../meter/index.ts";
import type { PaymentProvider } from "../provider/index.ts";
import { serveFinance } from "../server/index.ts";
import { Sku } from "../sku/index.ts";
import { financeService } from "../service/index.ts";
import { financeDatabase } from "../stack/index.ts";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The residency of the accounts the fixture copies. */
const RESIDENCY = "eu";

/** How long a test caller's authentication lasts, in milliseconds: a placement token's minute. */
const LIFETIME_MILLISECONDS = 60_000;

/** The origin the fixture's clients call. */
const ORIGIN = "https://finance.test";

/** A use of a resource a test routes to an account, as a host measured it. */
export interface FinanceUse {
    /** The use's identity within its source. */
    readonly id: string;
    /** The scope the use happened in, such as a space. */
    readonly source: string;
    /** The meter measuring it. */
    readonly meter: CatalogReference;
    /** The amount used in the meter's unit. */
    readonly quantity: number;
    /** The level kept, for an averaged meter. */
    readonly level?: number;
    /** The SKU rating it, absent for a seller's own meter. */
    readonly sku?: CatalogReference;
    /** The meter's unit. */
    readonly unit?: string;
    /** When the use was measured, in UTC epoch milliseconds. */
    readonly time: number;
}

/** The options of a finance fixture. */
export interface FinanceFixtureOptions {
    /** The payment provider billing through, none by default. */
    readonly provider?: PaymentProvider;
    /** Read the current time the service runs at, the test's clock, the system clock by default. */
    readonly clock?: () => number;
}

/** The finance service over a migrated test database, serving clients that authenticate by a person's name. */
export class FinanceFixture {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly server: ObjectServer<ReturnType<typeof serveFinance>>;
    /** The HTTP server clients call. */
    readonly http: Server;
    /** The event service taking routed usage. */
    readonly eventHttp: Server;
    /** The latest release of each package, by identifier. */
    readonly #releases: Map<PackageId, Promise<BuildReader>>;
    /** Read the current time the service runs at. */
    readonly clock: () => number;
    /** The usage routed to the accounts, kept over the test database and a memory bucket. */
    readonly events: EventStore;
    /** The usage read back from the events. */
    readonly meterUsage: MeterUsage;
    /** The rating of each account's usage, which the test runs where the control loop would. */
    readonly #rating: UsageController;
    /** The derivation of each account's entitlements, which the test runs where the control loop would. */
    readonly #entitlements: EntitlementController;

    /** Serve finance over a database, with the releases its grants and meters read. */
    private constructor(
        test: TestDatabase,
        releases: Map<PackageId, Promise<BuildReader>>,
        options: FinanceFixtureOptions,
    ) {
        // serve the objects on the test's clock, opening each package's latest release
        this.test = test;
        this.#releases = releases;
        this.clock = options.clock ?? Date.now;
        const bucket = new MemoryBucket(this.clock);
        this.events = new EventStore({
            database: test.database,
            kinds: [usage],
            files: () => bucket,
            now: this.clock,
        });
        this.meterUsage = new MeterUsage(this.events);
        const catalog = Catalog.tagged(
            { open: (packageId) => found(this.#releases, packageId) },
            LATEST_TAG,
        );
        this.server = new ObjectServer({
            objects: serveFinance(catalog, options.provider),
            policies: [account, organisation],
            database: test.database,
            callKey: testCallKey,
            origin: { package: financeService.package, service: financeService.name },
            shapes: [allowanceShape, universeAllowanceShape],
            clock: this.clock,
        });

        // rate usage and derive entitlements as the controllers do
        this.#rating = new UsageController(this.server, this.meterUsage, catalog, () => undefined);
        this.#entitlements = new EntitlementController(this.server, catalog, this.meterUsage);

        // answer clients authenticating by a person's name, leaving the controllers to the test
        this.http = Server.start({
            ...this.server.implement(financeService, []),
            controllers: [],
            audience: financeService.package.id,
            drainTimeout: 1000,
            health: new Health("finance"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) => this.#authenticate(request),
        });
        this.eventHttp = Server.start({
            ...implementEvents({
                store: this.events,
                access: this.server.access,
                admit: admitUsage(this.server),
            }),
            controllers: [],
            audience: financeService.package.id,
            drainTimeout: 1000,
            health: new Health("finance"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) => this.#authenticate(request),
        });
    }

    /** Receive an account's uses, then rate them page by page. */
    async use(scope: string, uses: readonly FinanceUse[]): Promise<void> {
        // receive the uses as the account's usage events
        await this.events.receive(
            usage.key,
            uses.map((use) => ({
                scope,
                id: use.id,
                source: use.source,
                time: use.time * 1000,
                keys: {
                    meter: CatalogReference.key(use.meter),
                    sku: use.sku === undefined ? null : CatalogReference.key(use.sku),
                    installation: null,
                    quantity: use.quantity,
                    level: use.level ?? null,
                },
                data: { unit: use.unit ?? "", from: use.time, to: use.time },
            })),
        );

        // rate every page
        while ((await this.#rating.reconcile(scope)) === 0) {
            continue;
        }
    }

    /** Open the service on a dialect with the releases of some packages, billing through a provider when given. */
    static async open(
        dialect: Dialect,
        releases: readonly (readonly [Package, readonly (Feature | Meter | Sku)[]])[],
        options: FinanceFixtureOptions = {},
    ): Promise<FinanceFixture> {
        const test = await TestDatabase.create(dialect, financeDatabase, { isMigrated: true });
        const built = new Map(
            releases.map(([owner, declared]) => [
                owner.id,
                FinanceFixture.release(owner, declared),
            ]),
        );

        return new FinanceFixture(test, built, options);
    }

    /** Build a release of a package declaring some features, meters and SKUs, as its build records them. */
    static async release(
        owner: Package,
        declared: readonly (Feature | Meter | Sku)[],
    ): Promise<BuildReader> {
        const declarations = declared.map((entry) => ({
            package: financeService.package.id,
            name: entry.name,
            ...FinanceFixture.#describe(entry),
        }));

        return (await MemoryBuild.declaring(owner, declarations)).reader;
    }

    /** Describe a declaration by its kind. */
    static #describe(entry: Feature | Meter | Sku) {
        if (entry instanceof Feature) {
            return { kind: "feature", description: describeFeature(entry) };
        } else if (entry instanceof Sku) {
            return { kind: "sku", description: describeSku(entry) };
        }

        return { kind: "meter", description: describeMeter(entry) };
    }

    /** Publish a later release of a package, which becomes its latest. */
    publish(owner: Package, declared: readonly (Feature | Meter | Sku)[]): void {
        this.#releases.set(owner.id, FinanceFixture.release(owner, declared));
    }

    /** Copy an account a person owns, as the account service replicates it. */
    async account(id: Identifier<"account">, handle: string, person: string): Promise<void> {
        // copy the person's user and the account below it
        const { database } = this.test;
        const copies = new AccessFixture(database);
        const owner = user.identifier(person);
        await copies.copyScope(user.reference(Scope.universe.id, owner));
        const reference = account.reference(owner, id);
        await copies.copyScope(reference);
        await database.insert(account.table).values({
            id,
            scope: owner,
            handle,
            name: handle,
            residencyId: "eu",
            createdAt: 0,
            updatedAt: 0,
        });

        // let the person own it, and the workloads of its residency serve it
        await copies.copyOwner(
            reference,
            Subject.parse(principal.user.reference(Scope.universe.id, owner)),
            this.clock(),
        );
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: reference,
                    relation: "resident",
                    subject: Account.resident(RESIDENCY),
                    createdAt: 0,
                    expiresAt: null,
                },
                owner,
            ),
        );
    }

    /** Open a client calling as a person. */
    client(person: string): ReturnType<typeof connect> {
        return this.#connect(`user ${person}`);
    }

    /** Open a client calling as a workload. */
    workload(
        name: string,
        claims: { readonly residencies?: readonly string[]; readonly lapsesAt?: number } = {},
    ): ReturnType<typeof connect> {
        const { residencies = [RESIDENCY], lapsesAt = this.clock() + LIFETIME_MILLISECONDS } =
            claims;

        return this.#connect(`workload ${name} ${residencies.join(",")} ${lapsesAt}`);
    }

    /** Open an event service client calling as a workload. */
    workloadEvents(
        name: string,
        claims: { readonly residencies?: readonly string[]; readonly lapsesAt?: number } = {},
    ) {
        const { residencies = [RESIDENCY], lapsesAt = this.clock() + LIFETIME_MILLISECONDS } =
            claims;

        return createClient(eventService, {
            url: ORIGIN,
            headers: {
                authorization: `Bearer workload ${name} ${residencies.join(",")} ${lapsesAt}`,
            },
            fetch: (request: Request) => this.eventHttp.fetch(request),
        });
    }

    /** Open a client calling with a bearer naming its principal. */
    #connect(bearer: string): ReturnType<typeof connect> {
        return connect({
            url: ORIGIN,
            headers: { authorization: `Bearer ${bearer}` },
            fetch: (request: Request) => this.http.fetch(request),
        });
    }

    /** Derive every account's entitlements and allowances, as the controller does after each change. */
    async entitle(): Promise<void> {
        for (const key of await this.#entitlements.list()) {
            await this.#entitlements.reconcile(key);
        }
    }

    /** Run an object type's controller over every pending key once. */
    async control(name: string): Promise<void> {
        const controller = this.server.controllers().find((each) => each.name === name);
        if (controller === undefined) {
            throw new TypeError(`the ${name} controller is missing`);
        }
        for (const key of await controller.list()) {
            await controller.reconcile(key, reconciliation());
        }
    }

    /** Stop serving and close the database. */
    async close(): Promise<void> {
        await this.http.close();
        await this.eventHttp.close();
        await this.test.close();
    }

    /** Authenticate a request as the user or workload its bearer names. */
    #authenticate(request: Request): Authentication {
        // read the bearer's user or workload
        const header = request.headers.get("authorization");
        if (header === null) {
            throw new TypeError("a test request carries no authorization");
        }
        const [kind, name = "", residencies = "", lapsesAt = ""] = header
            .slice("Bearer ".length)
            .split(" ");
        const now = this.clock();
        if (kind !== "workload") {
            const subject = principal.user.reference(Scope.universe.id, user.identifier(name));

            return new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: "user", id: subject.id },
                audience: financeService.package.id,
                verifiedAt: now,
                expiresAt: now + LIFETIME_MILLISECONDS,
            });
        }

        // verify a workload with the residencies its token claims, as a placement's token names them
        const subject = principal.workload.reference(Scope.universe.id, name);
        const claimed = residencies === "" ? [] : residencies.split(",");
        const expiresAt = Number(lapsesAt);

        return new Authentication({
            subject,
            subjects: [subject, ...claimed.map((code) => Account.resident(code))],
            credential: { kind: "workload", id: subject.id },
            audience: financeService.package.id,
            verifiedAt: Math.min(now, expiresAt - LIFETIME_MILLISECONDS),
            expiresAt,
        });
    }
}
