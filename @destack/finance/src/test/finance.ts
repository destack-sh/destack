import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/service/authentication";
import { allowanceShape, universeAllowanceShape } from "@destack/account/object";
import { accessRelationship, anyone, type Caller, principal, Relationship } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import {
    Account,
    account,
    organisation,
    residency,
    Residency,
    space,
    user,
} from "@destack/account/object";
import { type DatabaseConnection, type Dialect, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { LATEST_TAG } from "@destack/forge/object";
import { ObjectStore, Stack } from "@destack/object/server";
import { type Package, PackageId } from "@destack/package";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { Clock, found, Identifier, JsonValue, present, schema } from "@destack/schema";
import { Health } from "@destack/service/health";
import { Server } from "@destack/service/server";
import { reconciliation, TestBearer, testCallKey } from "@destack/service/test";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
import { v7 } from "uuid";
import { EventStore, EventTime } from "@destack/event";
import { MemoryBucket } from "@destack/bucket/test";
import { type Plan } from "@destack/resource";
import { StackDefinition } from "@destack/stack/declare";
import { usage } from "../meter/usage.ts";
import { homeDatabase } from "./home.ts";
import { EntitlementController } from "../server/entitlement.ts";
import { Metering } from "../meter/metering.ts";
import { UsageController } from "../server/usage.ts";
import { createClient, type ServiceClient } from "@destack/service/client";
import { Feature } from "../feature/index.ts";
import { describeFeature, describeMeter, describeSku } from "../inspect/index.ts";
import type { Meter } from "../meter/index.ts";
import type { PaymentProvider } from "../provider/index.ts";
import { serveFinance } from "../server/index.ts";
import { price, product, productFeature, seller } from "../object/index.ts";
import { Sku } from "../sku/index.ts";
import { financeService } from "../service/index.ts";
import { Catalog } from "../catalog/catalog.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The residency of the accounts the fixture copies. */
const RESIDENCY = "eu";

/** The platform's organisation, owning the residency's platform account. */
const PLATFORM_ORGANISATION = "organisation-019f5530-8000-7000-8000-0000000000f6";

/** The platform's account in the residency, whose home is the residency's. */
const PLATFORM_ACCOUNT = "account-019f5530-8000-7000-8000-0000000000f7";

/** The installation and package of the home's stack, which declares the platform's catalog. */
const STACK = {
    installationId: schema
        .identifier("installation")
        .parse("installation-019f5530-8000-7000-8000-0000000000fa"),
    packageId: PackageId.parse("package-019f5530-8000-7000-8000-0000000000fb"),
};

/** The names the platform's organisation keeps without accounts, as the directory reserves them. */
const PLATFORM_NAMES: ReadonlySet<string> = new Set(["platform"]);

/** The handle of the platform's account, the scope of the platform's packages. */
const PLATFORM_HANDLE = "destack";

/** The residency's home space, the platform account's home, where finance keeps its rows. */
const HOME = Account.home(PLATFORM_ACCOUNT);

/** The finance installation serving the residency, in the residency's home space. */
const INSTALLATION = principal.installation.reference(
    HOME,
    "installation-019f5530-8000-7000-8000-0000000000f9",
);

/** The origin the fixture's clients call. */
const ORIGIN = "https://finance.test";

/** A use of a resource a test routes to an account, as a machine measured it. */
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
    /** Open the payment provider billing through for the home, none by default. */
    readonly provider?: (home: string) => PaymentProvider;
    /** The clock the service runs at, the system clock by default. */
    readonly clock?: Clock;
    /** Report a use its declarations refuse, failing the test by default. */
    readonly report?: (error: unknown) => void;
}

/** The finance service over a migrated test database, serving clients that sign in by name. */
export class FinanceFixture {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly store: ObjectStore;
    /** The HTTP server clients call. */
    readonly http: Server;
    /** The latest release of each package, by identifier. */
    readonly #releases: Map<PackageId, Promise<BuildReader>>;
    /** The clock the service runs at. */
    readonly clock: Clock;
    /** The usage routed to the accounts, kept over the test database and a memory bucket. */
    readonly events: EventStore;
    /** The usage metered against the releases' catalogs. */
    readonly metering: Metering;
    /** The payment provider billing through, absent for a fixture selling nothing. */
    readonly provider: PaymentProvider | undefined;
    /** The finance objects the store serves, with their handlers. */
    readonly #served: ReturnType<typeof serveFinance>;
    /** The rating of each account's usage, which the test runs where the control loop would. */
    readonly #rating: UsageController;
    /** The entitlement derivation, which the test runs where the control loop would. */
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
        this.clock = options.clock ?? Clock.system;
        const bucket = new MemoryBucket(this.clock);
        this.events = new EventStore({
            database: test.database,
            kinds: [usage],
            files: () => bucket,
            clock: this.clock,
            // fail the test on a dropped event
            report: (error) => {
                throw error;
            },
        });
        const catalog = Catalog.tagged(
            { open: (packageId) => found(this.#releases, packageId) },
            LATEST_TAG,
        );
        // fail the test on a refused use unless it reports them
        this.metering = new Metering(
            this.events,
            catalog,
            (handle) => FinanceFixture.#publisher(Snapshot.live(test.database), handle),
            options.report ??
                ((error) => {
                    throw error;
                }),
        );
        this.provider = options.provider === undefined ? undefined : options.provider(HOME);
        this.#served = serveFinance(this.metering, this.provider, HOME);
        this.store = new ObjectStore({
            principal: INSTALLATION,
            objects: this.#served,
            policies: [account, organisation, residency],
            database: test.database,
            callKey: testCallKey,
            origin: { package: financeService.package, service: financeService.name },
            shapes: [allowanceShape, universeAllowanceShape],
            clock: this.clock,
        });

        // rate usage and derive entitlements as the controllers do
        this.#rating = new UsageController(this.store, HOME);
        this.#entitlements = new EntitlementController(this.store, HOME);

        // answer clients authenticating by a person's name, leaving the controllers to the test
        this.http = Server.start({
            ...this.store.implement(
                financeService,
                this.store.routed,
                this.store.controllers(),
                {},
            ),
            controllers: [],
            clock: this.clock,
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
        // receive the uses, then rate every page
        await this.receive(scope, uses);
        while ((await this.rate(scope)) === 0) {
            continue;
        }
    }

    /** Receive an account's uses as its usage events, leaving them unrated. */
    async receive(scope: string, uses: readonly FinanceUse[]): Promise<void> {
        await this.events.receive(
            usage.key,
            uses.map((use) => ({
                scope,
                id: use.id,
                source: use.source,
                time: EventTime.of(use.time),
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
    }

    /** Rate the next page of an account's usage as the controller does, while pages are full. */
    rate(scope: string): Promise<number | undefined> {
        return this.#rating.reconcile(scope);
    }

    /** Open the service on a dialect with package releases, billing through a provider if given. */
    static async open(
        dialect: Dialect,
        releases: readonly (readonly [Package, readonly (Feature | Meter | Sku)[]])[],
        options: FinanceFixtureOptions = {},
    ): Promise<FinanceFixture> {
        // migrate the database, serving the residency, and build each package's release
        const test = await TestDatabase.create(dialect, homeDatabase, { isMigrated: true });
        await FinanceFixture.#serveResidency(test.database);
        await FinanceFixture.#copyHome(test.database);
        const built = new Map(
            releases.map(([owner, declared]) => [
                owner.id,
                FinanceFixture.release(owner, declared),
            ]),
        );

        return new FinanceFixture(test, built, options);
    }

    /** Relate the finance installation as a residency server, as the platform binds it. */
    static async #serveResidency(database: DatabaseConnection): Promise<void> {
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: residency.reference(Scope.universe.id, RESIDENCY),
                    relation: "installation",
                    subject: INSTALLATION,
                    createdAt: 0,
                    expiresAt: null,
                },
                Scope.universe.id,
            ),
        );
    }

    /** Copy the residency's public home space below the platform account, as replication does. */
    static async #copyHome(database: DatabaseConnection): Promise<void> {
        // copy the platform's organisation, its account and the home below them
        const copies = new AccessFixture(database);
        const owner = organisation.reference(Scope.universe.id, PLATFORM_ORGANISATION);
        const platform = account.reference(PLATFORM_ORGANISATION, PLATFORM_ACCOUNT);
        const home = space.reference(PLATFORM_ACCOUNT, HOME);
        await copies.copyScope(owner);
        await copies.copyScope(platform);
        await copies.copyScope(home, "own");

        // show the home to anyone
        await copies.copyRelationship(HOME, home, "public", anyone.reference("*", "*"));
    }

    /** The residency's home space, where finance keeps its rows. */
    get home(): string {
        return HOME;
    }

    /** Apply a stack's sellers and products to the home, as the home's stack applies them. */
    declare(definition: Pick<StackDefinition, "sellers" | "products">): Promise<Plan> {
        const declared = [seller, product, price, productFeature];

        return Stack.apply({
            database: this.test.database,
            objects: declared.map((type) =>
                present(
                    this.#served.find((each) => each.same(type)),
                    `the served ${type.name}`,
                ),
            ),
            manager: STACK,
            as: principal.space.reference(Scope.universe.id, HOME),
            scope: HOME,
            now: this.clock.now(),
            document: StackDefinition.parse(JsonValue.of(definition)),
            store: this.store,
        });
    }

    /** Let a person operate the platform, owning the organisation whose account sells its plans. */
    async operate(person: string): Promise<Identifier<"account">> {
        // keep the platform's account under its handle, which the platform's packages carry
        const { database } = this.test;
        const platform = account.identifier(PLATFORM_ACCOUNT);
        await database.insert(account.table).values({
            id: platform,
            scope: PLATFORM_ORGANISATION,
            handle: PLATFORM_HANDLE,
            name: PLATFORM_HANDLE,
            residencyId: "eu",
            createdAt: 0,
            updatedAt: 0,
        });

        // let the person own the organisation, and so the account and its home
        const owner = principal.user.reference(Scope.universe.id, user.identifier(person));
        await new AccessFixture(database).copyOwner(
            organisation.reference(Scope.universe.id, PLATFORM_ORGANISATION),
            Subject.parse(owner),
            this.clock.now(),
        );

        // let the servers of its residency serve it
        await FinanceFixture.#reside(
            database,
            account.reference(PLATFORM_ORGANISATION, PLATFORM_ACCOUNT),
        );

        return platform;
    }

    /** Find the account selling a handle's packages to the home's buyers, as the directory does. */
    static async #publisher(
        snapshot: Snapshot,
        handle: string,
    ): Promise<Identifier<"account"> | null> {
        // sell the names the platform's organisation keeps without accounts through the platform
        if (PLATFORM_NAMES.has(handle)) {
            return account.identifier(PLATFORM_ACCOUNT);
        }

        // find the account with the handle, the platform's when its organisation keeps it
        const [held] = await snapshot.rows(account.table, { handle });
        if (held === undefined) {
            return null;
        }

        return held.scope === PLATFORM_ORGANISATION
            ? account.identifier(PLATFORM_ACCOUNT)
            : held.id;
    }

    /** Let the servers of the residency serve an account, as its residents. */
    static async #reside(database: DatabaseConnection, reference: ObjectReference): Promise<void> {
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: reference,
                    relation: "resident",
                    subject: Residency.servers(RESIDENCY),
                    createdAt: 0,
                    expiresAt: null,
                },
                reference.scope,
            ),
        );
    }

    /** Build a package release declaring some features, meters and SKUs, as its build does. */
    static async release(
        owner: Package,
        declared: readonly (Feature | Meter | Sku)[],
    ): Promise<BuildReader> {
        const declarations = declared.map((entry) => ({
            package: financeService.package.id,
            name: entry.name,
            ...FinanceFixture.#describe(entry),
        }));

        return await MemoryBuild.declaring(owner, declarations);
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

        // let the person own it, and the servers of its residency serve it
        await copies.copyOwner(
            reference,
            Subject.parse(principal.user.reference(Scope.universe.id, owner)),
            this.clock.now(),
        );
        await FinanceFixture.#reside(database, reference);
    }

    /** Open a client calling as a person. */
    client(person: string): ServiceClient<typeof financeService> {
        return this.#connect(`user ${person}`);
    }

    /** Open a client calling as a cloud machine serving residencies. */
    machine(
        name: string,
        claims: { readonly residencies?: readonly string[]; readonly lapsesAt?: number } = {},
    ): ServiceClient<typeof financeService> {
        const {
            residencies = [RESIDENCY],
            lapsesAt = this.clock.now() + TOKEN_LIFETIME_MILLISECONDS,
        } = claims;

        return this.#connect(`machine ${name} ${residencies.join(",")} ${lapsesAt}`);
    }

    /** Open a client calling with a bearer of its principal. */
    #connect(bearer: string): ServiceClient<typeof financeService> {
        return createClient(financeService, {
            url: ORIGIN,
            headers: { authorization: `Bearer ${bearer}` },
            fetch: (request: Request) => this.http.fetch(request),
        });
    }

    /** Derive every account's entitlements and allowances, as the controller does. */
    async entitle(): Promise<void> {
        for (const key of await this.#entitlements.list()) {
            await this.#entitlements.reconcile(key);
        }
    }

    /** Run an object type's controller over every pending key once. */
    async control(name: string): Promise<void> {
        const controller = this.store.controllers().find((each) => each.name === name);
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
        await this.test.close();
    }

    /** Authenticate a request as the user or cloud machine of its bearer. */
    #authenticate(request: Request): Caller {
        // read the bearer's user or cloud machine
        const bearer = TestBearer.read(request);
        const [kind, name = "", residencies = "", lapsesAt = ""] = bearer.split(" ");
        const now = this.clock.now();
        if (kind !== "machine") {
            const subject = principal.user.reference(Scope.universe.id, user.identifier(name));

            return TestBearer.authenticate(subject, financeService.package.id, now);
        }

        // verify a cloud machine with the residencies it serves
        const subject = principal.machine.reference(Scope.universe.id, name);
        const claimed = residencies === "" ? [] : residencies.split(",");
        const expiresAt = Number(lapsesAt);

        return {
            subject,
            subjects: [subject, ...claimed.map((code) => Residency.servers(code))],
            credential: {
                kind: "machine-key",
                id: subject.id,
                audience: financeService.package.id,
                verifiedAt: Math.min(now, expiresAt - TOKEN_LIFETIME_MILLISECONDS),
                expiresAt,
            },
        };
    }
}
