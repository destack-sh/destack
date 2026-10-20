import { DeclarationReference } from "@destack/package/declare";
import { STACKS, TestEntitlement } from "@destack/account/test";
import { TestTelemetry } from "@destack/telemetry/test";
import { TOKEN_LIFETIME_MILLISECONDS } from "@destack/runtime/authentication";
import { AccessContext, anyone, Authentication, Caller, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import {
    Account,
    account,
    claim,
    entitlement,
    machine,
    organisation,
    residency,
    Residency,
    space,
    user,
} from "@destack/account/object";
import { type DatabaseConnection, type Dialect, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { LocaleTag } from "@destack/locale";
import { notification } from "@destack/notification";
import notificationServer from "@destack/notification/server";
import { NotificationFixture } from "@destack/notification/test";
import { EventView, type ObjectType } from "@destack/object";
import { packageObject, tag } from "@destack/forge/object";
import { publishBuild, resolvedRelease } from "@destack/forge/test";
import { defineServer, ObjectStore, type PackageServer, Stack } from "@destack/object/server";
import { Package, PackageId } from "@destack/package";
import { type BuildReader } from "@destack/package/manifest";
import { MemoryBuild } from "@destack/package/test";
import { ResourceContext } from "@destack/resource/context";
import { Digest, Identifier, JsonValue, present, schema } from "@destack/schema";
import { Health } from "@destack/runtime/health";
import { RequestId } from "@destack/runtime/request";
import { Server } from "@destack/runtime/server";
import { reconcileOnce, reconciliation, TestBearer, testCallKey } from "@destack/runtime/test";
import { type ObjectReference, Scope, Subject } from "@destack/sync";
import { type EventStore, EventTime } from "@destack/event";
import { serversOf, testEvents } from "@destack/object/test";
import { type Plan } from "@destack/resource";
import { StackDefinition } from "@destack/stack/declare";
import { buildIndexServers } from "@destack/stack/test";
import { homeDatabase } from "./home.ts";
import { EntitlementController } from "../server/entitlement-controller.ts";
import { Metering } from "../server/metering.ts";
import { UsageController } from "../server/usage-controller.ts";
import { createClient, type ClientOf } from "@destack/runtime/client";
import { Feature } from "../feature/index.ts";
import { describeFeature, describeSku } from "../inspect/index.ts";
import { describeMeter } from "@destack/observability/inspect";
import observabilityPackage from "@destack/observability/package";
import type { Meter } from "@destack/observability/meter";
import type { Period } from "../rate/index.ts";
import type { PaymentProvider } from "../provider/index.ts";
import { implementFinance } from "../server/index.ts";
import {
    price,
    product,
    productFeature,
    seller,
    type Resolution,
    usage,
    Usage,
    usageLimit,
} from "../object/index.ts";
import { Sku } from "../sku/index.ts";
import financePackage from "../package.ts";
import { Clock } from "@destack/environment";

/** The residency of the accounts the fixture replicates. */
const RESIDENCY = "eu";

/** The tag the fixture opens releases at, which answers each package's latest. */
const RELEASE_TAG = "latest";

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

/** The names the platform's organisation reserves without accounts, as the directory does. */
const PLATFORM_NAMES: ReadonlySet<string> = new Set(["platform"]);

/** The handle of the platform's account, the scope of the platform's packages. */
const PLATFORM_HANDLE = "destack";

/** The residency's home space, the platform account's home, where finance stores its rows. */
const HOME = Account.home(PLATFORM_ACCOUNT);

/** The finance installation serving the residency, in the residency's home space. */
const INSTALLATION = principal.installation.reference(
    HOME,
    "installation-019f5530-8000-7000-8000-0000000000f9",
);

/** The origin the fixture's clients call. */
const ORIGIN = "https://finance.test";

/** Finance's build, declaring its notifications, as the residency's home installs it. */
const FINANCE_BUILD = await MemoryBuild.of(usageLimit.package, [
    {
        kind: "notification",
        kindPackageId: notification.package.id,
        name: usageLimit.name,
        description: usageLimit.describe(),
    },
]);

/** The machine serving the scenarios' spaces, which sends their uses. */
const SERVING_MACHINE = "machine-019f5530-8000-7000-8000-0000000000c5";

/** The spaces the scenarios' buyers pay for. */
export const spaces = {
    notes: schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000a1"),
    docs: schema.identifier("space").parse("space-019f5530-8000-7000-8000-0000000000b2"),
};

/** The dedicated machines the scenarios' buyers pay for. */
export const machines = {
    alpha: schema.identifier("machine").parse("machine-019f5530-8000-7000-8000-0000000000a3"),
    beta: schema.identifier("machine").parse("machine-019f5530-8000-7000-8000-0000000000b4"),
};

/** A use of a resource in a space an account pays for, as its machine measured it. */
export interface FinanceUse {
    /** The use's identity, unique across spaces. */
    readonly id: string;
    /** The space the use ran in, absent for a dedicated machine's own hours. */
    readonly space?: Identifier<"space">;
    /** The dedicated machine whose hours were used, absent for a space's use. */
    readonly machine?: Identifier<"machine">;
    /** The meter measuring it. */
    readonly meter: DeclarationReference;
    /** The amount used in the meter's unit. */
    readonly quantity: number;
    /** The level held, for an averaged meter. */
    readonly level?: number;
    /** The SKU rating it, absent for a seller's own meter. */
    readonly sku?: DeclarationReference;
    /** When the use was measured, in UTC epoch milliseconds. */
    readonly time: number;
}

/** The options of a finance fixture. */
export interface FinanceFixtureOptions {
    /** Open the payment provider billing through for the home, none by default. */
    readonly provider?: (home: string) => PaymentProvider;
    /** The clock finance runs at, the system clock by default. */
    readonly clock?: Clock;
}

/** Finance over a migrated test database, serving clients that sign in by name. */
export class FinanceFixture {
    /** The isolated database. */
    readonly test: TestDatabase;
    /** The served finance objects. */
    readonly store: ObjectStore;
    /** The HTTP server clients call. */
    readonly http: Server;
    /** The clock finance runs at. */
    readonly clock: Clock;
    /** The event log recording the calls and the payers' usage. */
    readonly events: EventStore;
    /** The usage metered against the releases' catalogs. */
    readonly metering: Metering;
    /** The payment provider billing through, absent for a fixture selling nothing. */
    readonly provider: PaymentProvider | undefined;
    /** The finance objects the store serves, with their handlers. */
    readonly #served: readonly ObjectType[];
    /** The rating of each account's usage, which the test runs where the control loop would. */
    readonly #rating: UsageController;
    /** The telemetry recording the exceptions finance captures, refused at close unless taken. */
    readonly telemetry = TestTelemetry.register();
    /** The spaces and machines replicated below their payers so far. */
    readonly #replicated = new Set<string>();
    /** The entitlement derivation, which the test runs where the control loop would. */
    readonly #entitlements: EntitlementController;
    /** The published builds the metering reads, by their manifest's digest. */
    readonly #published: Map<string, BuildReader>;

    /** Serve finance over a database, with the releases its grants and meters read. */
    private constructor(
        test: TestDatabase,
        events: EventStore,
        index: readonly PackageServer[],
        published: Map<string, BuildReader>,
        options: FinanceFixtureOptions,
    ) {
        // serve the objects on the test's clock, resolving each package's latest release in forge
        this.test = test;
        this.#published = published;
        this.clock = options.clock ?? Clock.system;
        this.events = events;

        // meter the usage against the releases, publishing each package's SKUs as its account
        this.metering = new Metering(
            RELEASE_TAG,
            (handle) => FinanceFixture.#publisher(Snapshot.live(test.database), handle),
            async (_packageId, manifest) =>
                present(published.get(manifest), `the published build ${manifest}`),
        );
        this.provider = options.provider === undefined ? undefined : options.provider(HOME);
        const definition = implementFinance(HOME, this.metering, this.provider);
        this.#served = definition.objects ?? [];
        this.store = new ObjectStore({
            servers: [
                defineServer(financePackage, definition),
                notificationServer,
                ...serversOf([packageObject, resolvedRelease, tag, entitlement]),
                ...index,
            ],
            chain: async () => undefined,
            principal: INSTALLATION,
            policies: [account, organisation, residency],
            claims: { object: claim },
            database: test.database,
            // follow the universe's entitlements through a publisher that sends nothing, as no universe runs here
            publisher: () => TestEntitlement.publisher,
            events: this.events,
            callKey: testCallKey,
            origin: { package: Package.of(financePackage) },
            clock: this.clock,
        });

        // rate usage and derive entitlements as the controllers do
        this.#rating = new UsageController(this.store, HOME);
        this.#entitlements = new EntitlementController(this.store, HOME, this.metering);

        // answer clients authenticating by a person's name, leaving the controllers to the test
        this.http = Server.start({
            ...this.store.package(financePackage),
            controllers: [],
            clock: this.clock,
            drainTimeout: 1000,
            health: new Health("finance"),
            resources: new ResourceContext(),
            authorizeMachine: async () => {},
            authenticate: async (request) => this.#authenticate(request),
        });
    }

    /** Append an account's uses, then rate them page by page. */
    async use(payer: string, uses: readonly FinanceUse[]): Promise<void> {
        // append the uses, then rate every page
        await this.append(payer, uses);
        while ((await this.rate(payer)) === 0) {
            continue;
        }
    }

    /** Fold a paying account's usage of a meter as rating reads it, in a period or now. */
    usage(payer: string, meter: Meter, period: Period | undefined): Promise<number> {
        const payerId = account.identifier(payer);

        const events = new EventView(usage.eventKind(), this.events);

        return Usage.read(payerId, meter, period, this.clock.now(), events);
    }

    /** Append an account's uses as its spaces and machines send them, leaving them unrated. */
    async append(payer: string, uses: readonly FinanceUse[]): Promise<void> {
        // send a space's uses as the residency's servers, and a machine's as the machine itself
        const now = this.clock.now();
        const payerId = account.identifier(payer);
        for (const [machineId, sent] of Map.groupBy(uses, (use) => use.machine ?? null)) {
            const sender = await this.#sender(payerId, machineId, sent);
            await this.store.mutate(AccessContext.of(Caller.of(sender), now), {
                id: RequestId.create(now),
                calls: [
                    usage.call("append", {
                        ...usage.in(payerId),
                        items: sent.map((use) => FinanceFixture.#item(use)),
                    }),
                ],
            });
        }
    }

    /** Resolve each feature an account's sources grant now, with its metered usage. */
    resolutions(scope: string): Promise<Resolution[]> {
        return this.#entitlements.resolve(scope);
    }

    /** Rate the next page of an account's usage and derive its entitlements, as the loop does. */
    async rate(scope: string): Promise<number | undefined> {
        const rated = await this.#rating.reconcile(scope);
        await this.#entitlements.reconcile(scope);

        return rated;
    }

    /** Flush the hot usage of an account into segments, as the event store's controller does. */
    async flush(payer: string): Promise<void> {
        for (const controller of this.events.controllers) {
            await controller.reconcile(`${usage.eventKind().key} ${payer}`, reconciliation());
        }
    }

    /** Rate each payer a restarted usage controller lists until its pages run out. */
    async rateListed(): Promise<readonly string[]> {
        const listed = await this.#rating.list();
        for (const payer of listed) {
            while ((await this.rate(payer)) === 0) {
                continue;
            }
        }

        return listed;
    }

    /** Replicate the spaces or machine of uses below their payer once, returning their sender. */
    async #sender(
        payer: Identifier<"account">,
        machineId: Identifier<"machine"> | null,
        uses: readonly FinanceUse[],
    ): Promise<Subject> {
        // replicate each space below the payer once, served by a machine of the payer
        const access = new AccessFixture(this.test.database);
        if (machineId === null) {
            for (const use of uses) {
                const spaceId = present(use.space, `the space of ${use.id}`);
                if (!this.#replicated.has(spaceId)) {
                    this.#replicated.add(spaceId);
                    await this.test.database.insert(space.table).values({
                        id: spaceId,
                        scope: payer,
                        name: `space-${this.#replicated.size}`,
                        stack: STACKS.space,
                        createdAt: 0,
                        updatedAt: 0,
                    });
                    await access.replicateScope(space.reference(payer, spaceId), "own");
                }
            }

            return Subject.parse(principal.machine.reference(payer, SERVING_MACHINE));
        }

        // replicate a machine below the payer once, the machine itself sending its uses
        const used = machine.reference(payer, machineId);
        if (!this.#replicated.has(machineId)) {
            this.#replicated.add(machineId);
            await this.test.database.insert(machine.table).values({
                id: machineId,
                scope: payer,
                name: `machine-${this.#replicated.size}`,
                kind: "personal",
                platform: "Linux",
                architecture: "arm64",
                model: "test",
                createdAt: 0,
                updatedAt: 0,
            });
            await access.replicateScope(used);
        }

        return Subject.parse(principal.machine.reference(Scope.universe.id, machineId));
    }

    /** Write a use as the usage append method takes it. */
    static #item(use: FinanceUse) {
        return {
            id: use.id,
            time: EventTime.of(use.time),
            meter: DeclarationReference.key(use.meter),
            sku: use.sku === undefined ? null : DeclarationReference.key(use.sku),
            spaceId: use.space ?? null,
            machineId: use.machine ?? null,
            installationId: null,
            quantity: use.quantity,
            level: use.level ?? null,
            data: { from: use.time, to: use.time },
        };
    }

    /** Open finance on a dialect with package releases, billing through a provider if given. */
    static async open(
        dialect: Dialect,
        releases: readonly (readonly [Package, readonly (Feature | Meter | Sku)[]])[],
        options: FinanceFixtureOptions = {},
    ): Promise<FinanceFixture> {
        // migrate the database, serving the residency, and build each package's release
        const test = await TestDatabase.create(dialect, homeDatabase, { isMigrated: true });
        await FinanceFixture.#serveResidency(test.database);
        await FinanceFixture.#replicateHome(test.database);
        await NotificationFixture.install(test.database, HOME, [FINANCE_BUILD]);
        const published = new Map<string, BuildReader>();
        for (const [owner, declared] of releases) {
            const build = await FinanceFixture.release(owner, declared);
            await publishBuild(test.database, HOME, account.identifier(PLATFORM_ACCOUNT), build);
            published.set(await Digest.json(build.manifest), build);
        }

        // record the calls and the usage in the database's event log
        const clock = options.clock ?? Clock.system;
        const events = await testEvents(test.database, [usage.eventKind()], clock);

        // serve the index of the finance build the home installs
        const index = await buildIndexServers(test.database, [FINANCE_BUILD]);

        return new FinanceFixture(test, events, index, published, options);
    }

    /** Relate the finance installation as a residency server, as the platform binds it. */
    static async #serveResidency(database: DatabaseConnection): Promise<void> {
        await new AccessFixture(database).replicateRelationship(
            Scope.universe.id,
            residency.reference(Scope.universe.id, RESIDENCY),
            "installation",
            INSTALLATION,
            0,
        );
    }

    /** Replicate the residency's public home space below the platform account. */
    static async #replicateHome(database: DatabaseConnection): Promise<void> {
        // replicate the platform's organisation, its account and the home below them
        const access = new AccessFixture(database);
        const owner = organisation.reference(Scope.universe.id, PLATFORM_ORGANISATION);
        const platform = account.reference(PLATFORM_ORGANISATION, PLATFORM_ACCOUNT);
        const home = space.reference(PLATFORM_ACCOUNT, HOME);
        await access.replicateScope(owner);
        await access.replicateScope(platform);
        await database.insert(space.table).values({
            id: HOME,
            scope: account.identifier(PLATFORM_ACCOUNT),
            name: "home",
            stack: STACKS.space,
            createdAt: 0,
            updatedAt: 0,
        });
        await access.replicateScope(home, "own");

        // show the home to anyone from the start, as the platform's standing rows are
        await access.replicateRelationship(HOME, home, "public", anyone.reference("*", "*"), 0);
    }

    /** The residency's home space, where finance stores its rows. */
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
        // create the platform's account under its handle, which the platform's packages carry
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
        await new AccessFixture(database).bindOwner(
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
        // sell the names the platform's organisation reserves without accounts through the platform
        if (PLATFORM_NAMES.has(handle)) {
            return account.identifier(PLATFORM_ACCOUNT);
        }

        // find the account with the handle, the platform's when its organisation reserves it
        const [found] = await snapshot.read(account.table).findMany({ where: { handle } });
        if (found === undefined) {
            return null;
        }

        return found.scope === PLATFORM_ORGANISATION
            ? account.identifier(PLATFORM_ACCOUNT)
            : found.id;
    }

    /** Let the servers of the residency serve an account, as its residents. */
    static async #reside(database: DatabaseConnection, reference: ObjectReference): Promise<void> {
        await new AccessFixture(database).replicateRelationship(
            reference.scope,
            reference,
            "resident",
            Residency.servers(RESIDENCY),
            0,
        );
    }

    /** Build a package release declaring some features, meters and SKUs, as its build does. */
    static async release(
        owner: Package,
        declared: readonly (Feature | Meter | Sku)[],
    ): Promise<BuildReader> {
        const declarations = declared.map((entry) => ({
            name: entry.name,
            ...FinanceFixture.#describe(entry),
        }));

        return await MemoryBuild.of(owner, declarations);
    }

    /** Describe a declaration by its kind and the package declaring the kind. */
    static #describe(entry: Feature | Meter | Sku) {
        if (entry instanceof Feature) {
            return {
                kindPackageId: financePackage.id,
                kind: "feature",
                description: describeFeature(entry),
            };
        } else if (entry instanceof Sku) {
            return {
                kindPackageId: financePackage.id,
                kind: "sku",
                description: describeSku(entry),
            };
        }

        return {
            kindPackageId: observabilityPackage.id,
            kind: "meter",
            description: describeMeter(entry),
        };
    }

    /** Publish a later release of a package, which becomes its latest. */
    async publish(owner: Package, declared: readonly (Feature | Meter | Sku)[]): Promise<void> {
        const build = await FinanceFixture.release(owner, declared);
        await publishBuild(
            this.test.database,
            HOME,
            account.identifier(PLATFORM_ACCOUNT),
            build,
            this.clock.now(),
        );
        this.#published.set(await Digest.json(build.manifest), build);
    }

    /** Replicate an account a person owns, as the account package replicates it. */
    async account(id: Identifier<"account">, handle: string, person: string): Promise<void> {
        // replicate the person's user and the account below it
        const { database } = this.test;
        const access = new AccessFixture(database);
        const owner = user.identifier(person);
        await access.replicateScope(user.reference(Scope.universe.id, owner));
        await NotificationFixture.person(
            database,
            principal.user.reference(Scope.universe.id, owner),
            { name: person, locale: LocaleTag.parse("en"), timeZone: "UTC" },
        );
        const reference = account.reference(owner, id);
        await access.replicateScope(reference);
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
        await access.bindOwner(
            reference,
            Subject.parse(principal.user.reference(Scope.universe.id, owner)),
            this.clock.now(),
        );
        await FinanceFixture.#reside(database, reference);
    }

    /** Open a client calling as a person. */
    client(person: string): ClientOf<typeof financePackage> {
        return this.#connect(`user ${person}`);
    }

    /** Open a client calling as a cloud machine serving residencies. */
    machine(
        name: string,
        claims: { readonly residencies?: readonly string[]; readonly lapsesAt?: number } = {},
    ): ClientOf<typeof financePackage> {
        const {
            residencies = [RESIDENCY],
            lapsesAt = this.clock.now() + TOKEN_LIFETIME_MILLISECONDS,
        } = claims;

        return this.#connect(`machine ${name} ${residencies.join(",")} ${lapsesAt}`);
    }

    /** Open a client calling with a bearer of its principal. */
    #connect(bearer: string): ClientOf<typeof financePackage> {
        return createClient(financePackage, {
            url: ORIGIN,
            headers: { authorization: `Bearer ${bearer}` },
            fetch: (request: Request) => this.http.fetch(request),
        });
    }

    /** Derive every account's entitlements, as the controller does. */
    async entitle(): Promise<void> {
        for (const key of await this.#entitlements.list()) {
            await this.#entitlements.reconcile(key);
        }
    }

    /** Run an object type's controller over every pending key once. */
    async control(name: string): Promise<void> {
        const controller = present(
            this.store.controllers().find((each) => each.name === name),
            `the ${name} controller`,
        );
        await reconcileOnce(controller, this.store.database);
    }

    /** Stop serving and close the database, refusing exceptions no test took. */
    async close(): Promise<void> {
        await this.http.close();
        await this.test.close();
        this.telemetry[Symbol.dispose]();
    }

    /** Authenticate a request as the user or cloud machine of its bearer. */
    #authenticate(request: Request): Authentication {
        // read the bearer's user or cloud machine
        const bearer = TestBearer.read(request);
        const [kind, name = "", residencies = "", lapsesAt = ""] = bearer.split(" ");
        const now = this.clock.now();
        if (kind !== "machine") {
            const subject = principal.user.reference(Scope.universe.id, user.identifier(name));

            return TestBearer.authenticate(subject, financePackage.id, now);
        }

        // verify a cloud machine with the residencies it serves
        const subject = principal.machine.reference(Scope.universe.id, name);
        const claimed = residencies === "" ? [] : residencies.split(",");
        const expiresAt = Number(lapsesAt);

        return {
            caller: {
                ...Caller.of(subject),
                groups: claimed.map((code) => Residency.servers(code)),
            },
            credential: {
                kind: "machine-key",
                id: subject.id,
                audience: financePackage.id,
                verifiedAt: Math.min(now, expiresAt - TOKEN_LIFETIME_MILLISECONDS),
                expiresAt,
            },
        };
    }
}
